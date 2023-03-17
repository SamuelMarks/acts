use crate::{
    adapter::StoreAdapter,
    debug,
    sch::{self, ActId, NodeData, Scheduler},
    store::{none::NoneStore, Message, Proc, Query, Tag, Task},
    utils::{self, Id},
    ActResult, Engine, ShareLock, Vars, Workflow,
};
use std::sync::{Arc, Mutex, RwLock};

#[cfg(feature = "sqlite")]
use crate::store::db::SqliteStore;

#[cfg(feature = "store")]
use crate::store::db::LocalStore;

const LIMIT: usize = 10000;

fn tag_of(task: &sch::Task) -> Tag {
    match task.node.data {
        NodeData::Workflow(..) => Tag::Workflow,
        NodeData::Job(..) => Tag::Job,
        NodeData::Branch(..) => Tag::Branch,
        NodeData::Step(..) => Tag::Step,
        NodeData::Act(..) => Tag::Act,
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum StoreKind {
    None,
    #[cfg(feature = "store")]
    Local,
    #[cfg(feature = "sqlite")]
    Sqlite,
    Extern,
}

pub struct Store {
    kind: Arc<Mutex<StoreKind>>,
    base: ShareLock<Arc<dyn StoreAdapter>>,
}

impl Store {
    pub fn new() -> Self {
        #[allow(unused_assignments)]
        let mut store: Arc<dyn StoreAdapter> = Arc::new(NoneStore::new());
        #[allow(unused_assignments)]
        let mut kind = StoreKind::None;

        #[cfg(feature = "store")]
        {
            debug!("store::new");
            store = Arc::new(LocalStore::new());
            kind = StoreKind::Local;
        }

        #[cfg(feature = "sqlite")]
        {
            debug!("sqlite::new");
            store = Arc::new(SqliteStore::new());
            kind = StoreKind::Sqlite;
        }

        Self {
            kind: Arc::new(Mutex::new(kind)),
            base: Arc::new(RwLock::new(store)),
        }
    }

    pub fn init(&self, engine: &Engine) {
        debug!("store::init");
        if let Some(store) = engine.adapter().store() {
            *self.kind.lock().unwrap() = StoreKind::Extern;
            *self.base.write().unwrap() = store;
        }
    }

    pub fn flush(&self) {
        debug!("store::flush");
        self.base.read().unwrap().flush();
    }

    pub fn proc(&self, pid: &str, scher: &Scheduler) -> Option<sch::Proc> {
        debug!("store::proc({})", pid);
        let procs = self.base().procs();
        if let Some(p) = procs.find(pid) {
            let workflow = Workflow::from_str(&p.model).unwrap();
            // workflow.output_tree();
            let mut proc = scher.create_raw_proc(&workflow);
            proc.set_state(&p.state);
            self.load_proc_tasks(&mut proc);
            self.load_proc_messages(&mut proc);
            return Some(proc);
        }

        None
    }

    pub fn create_proc(&self, proc: &sch::Proc) {
        debug!("store::create_proc({})", proc.pid());
        let procs = self.base().procs();
        let workflow = &*proc.workflow();
        let data = Proc {
            id: proc.pid(), // pid is global unique id
            pid: proc.pid(),
            model: serde_yaml::to_string(workflow).unwrap(),
            state: proc.state(),
            vars: utils::vars::to_string(&proc.vm().vars()),
        };
        procs.create(&data).expect("store: create proc");
    }

    pub fn update_proc(&self, proc: &sch::Proc) {
        debug!("store::update_proc({})", proc.pid());
        let base = self.base();
        let procs = base.procs();

        let workflow = &*proc.workflow();
        let proc = Proc {
            id: proc.pid(), // pid is global unique id
            pid: proc.pid(),
            model: serde_yaml::to_string(workflow).unwrap(),
            state: proc.state(),
            vars: utils::vars::to_string(&proc.vm().vars()),
        };
        procs.update(&proc).expect("store: create proc");
    }

    pub fn remove_proc(&self, pid: &str) -> ActResult<bool> {
        debug!("store::remove_proc({})", pid);
        let base = self.base();
        let procs = base.procs();
        let tasks = base.tasks();
        let messages = base.messages();

        let query = Query::new().set_limit(LIMIT).push("pid", pid.into());
        for msg in messages.query(&query)? {
            messages.delete(&msg.id)?;
        }

        for task in tasks.query(&query)? {
            tasks.delete(&task.id)?;
        }
        procs.delete(pid)
    }

    pub fn create_task(&self, task: &sch::Task) {
        let tid = task.tid();
        let nid = task.nid();
        debug!("store::create_task({:?})", task);
        let tasks = self.base().tasks();
        let id = Id::new(&task.pid, &tid);
        let state = task.state();

        let task = Task {
            id: id.id(),
            tag: tag_of(&task),
            pid: task.pid.clone(),
            tid: tid,
            nid: nid,
            state,
            start_time: task.start_time(),
            end_time: task.end_time(),
            user: match task.uid() {
                Some(u) => u,
                None => "".to_string(),
            },
        };
        tasks.create(&task).expect("store: create task");
    }

    pub fn update_task(&self, task: &sch::Task, vars: &Vars) {
        debug!("store::update_task({})", task.tid());
        let procs = self.base().procs();
        let tasks = self.base().tasks();
        let pid = &task.pid;

        if let Some(mut proc) = procs.find(pid) {
            proc.vars = utils::vars::to_string(vars);
            procs.update(&proc).expect("store: update proc vars");
        }

        let tid = task.tid();
        let id = Id::new(pid, &tid);
        let state = task.state();
        if let Some(mut task) = tasks.find(&id.id()) {
            task.state = state;
            tasks.update(&task).expect("store: update task");
        }
    }

    pub fn create_message(&self, msg: &sch::Message) {
        debug!("store::create_message({})", msg.id);
        let base = self.base();
        let messages = base.messages();

        let uid = match &msg.uid {
            Some(uid) => uid,
            None => "",
        };
        messages
            .create(&Message {
                id: msg.id.clone(),
                pid: msg.pid.clone(),
                tid: msg.tid.clone(),
                user: uid.to_string(),
                create_time: msg.create_time,
            })
            .expect("store: create message");
    }

    pub fn load(&self, scher: Arc<Scheduler>, cap: usize) -> Vec<sch::Proc> {
        debug!("store::load({})", cap);
        let mut ret = Vec::new();
        if cap > 0 {
            let base = self.base();
            let procs = base.procs();
            let query = Query::new().set_limit(cap);
            // query.push("state", &TaskState::None.to_string());
            let items = procs.query(&query).expect("store: load");
            let mut iter = items.iter();
            while let Some(p) = iter.next() {
                let workflow = Workflow::from_str(&p.model).unwrap();
                let vars = &utils::vars::from_string(&p.vars);
                let mut proc = sch::Proc::new_raw(scher.clone(), &workflow, &p.pid, &p.state, vars);

                self.load_proc_tasks(&mut proc);
                self.load_proc_messages(&mut proc);

                ret.push(proc);
            }
        }

        ret
    }

    fn base(&self) -> Arc<dyn StoreAdapter> {
        self.base.read().unwrap().clone()
    }

    fn load_proc_tasks(&self, proc: &mut sch::Proc) {
        let base = self.base();
        let tasks = base.tasks();

        let query = Query::new().set_limit(LIMIT).push("pid", &proc.pid());
        let items = tasks.query(&query).expect("store: load_proc_tasks");
        for t in items {
            if t.tag == Tag::Workflow {
                proc.set_state(&t.state);
            }

            if let Some(node) = proc.node(&t.nid) {
                let task = sch::Task::new(&proc.pid(), &t.tid, node);
                task.set_state(&t.state);
                task.set_start_time(t.start_time);
                task.set_end_time(t.end_time);
                if !t.user.is_empty() {
                    task.set_uid(&t.user);
                }
                proc.push_task(Arc::new(task));
            }
        }
    }

    fn load_proc_messages(&self, proc: &mut sch::Proc) {
        let messages = self.base().messages();

        let query = Query::new().set_limit(LIMIT).push("pid", &proc.pid());
        let items = messages.query(&query).expect("store: load_proc_messages");
        for m in items {
            let uid = if m.user.is_empty() {
                None
            } else {
                Some(m.user)
            };
            proc.make_message(&m.tid, uid);
        }
    }

    #[cfg(test)]
    pub(crate) fn kind(&self) -> StoreKind {
        self.kind.lock().unwrap().clone()
    }
}
