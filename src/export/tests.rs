use crate::{
    data,
    event::{MessageState, Model},
    sch::TaskState,
    utils, Act, ActPlugin, ChannelOptions, Engine, Message, StoreAdapter, Vars, Workflow,
};
use serde_json::json;
use std::sync::{Arc, Mutex};

#[tokio::test]
async fn export_manager_publish_ok() {
    let engine = Engine::new();
    let manager = engine.manager();
    let pack = data::Package {
        id: "pack1".to_string(),
        name: "package 1".to_string(),
        file_data: vec![0x01, 0x02],
        ..Default::default()
    };

    let result = manager.publish(&pack);

    assert_eq!(result.is_ok(), true);
    assert_eq!(manager.publish(&pack).is_ok(), true);
}

#[tokio::test]
async fn export_manager_deploy_ok() {
    let engine = Engine::new();
    let manager = engine.manager();
    let model = Workflow::new()
        .with_id(&utils::longid())
        .with_step(|step| step.with_act(Act::req(|act| act.with_id("test"))));

    let result = manager.deploy(&model);

    assert_eq!(result.is_ok(), true);
    assert_eq!(manager.model(&model.id, "text").is_ok(), true);
}

#[tokio::test]
async fn export_manager_deploy_many_times() {
    let engine = Engine::new();
    let manager = engine.manager();
    let model = Workflow::new()
        .with_id(&utils::longid())
        .with_step(|step| step.with_id("step1"));

    let mut result = true;
    for _ in 0..10 {
        let state = manager.deploy(&model);
        result &= state.is_ok();
    }
    assert_eq!(result, true);
}

#[tokio::test]
async fn export_manager_deploy_no_model_id_error() {
    let engine = Engine::new();
    let manager = engine.manager();
    let model = Workflow::new().with_step(|step| step.with_id("step1"));

    let result = manager.deploy(&model);
    assert_eq!(result.is_err(), true);
}

#[tokio::test]
async fn export_manager_deploy_dup_id_error() {
    let engine = Engine::new();
    let manager = engine.manager();
    let model = Workflow::new()
        .with_id(&utils::longid())
        .with_step(|step| step.with_id("step1"))
        .with_step(|step| step.with_id("step1"));

    let result = manager.deploy(&model);
    assert_eq!(result.is_err(), true);
}

#[tokio::test]
async fn engine_executor_start_no_pid() {
    let engine = Engine::new();
    let executor = engine.executor();

    let mid = utils::longid();
    let workflow = Workflow::new()
        .with_id(&mid)
        .with_step(|step| step.with_act(Act::req(|act| act.with_id("test"))));
    engine.manager().deploy(&workflow).unwrap();
    let options = Vars::new();
    let result = executor.start(&workflow.id, &options);
    assert_eq!(result.is_ok(), true);
}

#[tokio::test]
async fn engine_executor_start_with_pid() {
    let engine = Engine::new();
    let executor = engine.executor();

    let mid = utils::longid();
    let workflow = Workflow::new()
        .with_id(&mid)
        .with_step(|step| step.with_act(Act::req(|act| act.with_id("test"))));
    engine.manager().deploy(&workflow).unwrap();
    let mut options = Vars::new();
    options.insert("pid".to_string(), "123".into());
    let result = executor.start(&workflow.id, &options);
    assert_eq!(result.is_ok(), true);

    assert_eq!(
        result.unwrap().outputs().get::<String>("pid").unwrap(),
        "123"
    );
}

#[tokio::test]
async fn export_executor_start_empty_pid() {
    let engine = Engine::new();
    let executor = engine.executor();

    let mid = utils::longid();
    let workflow = Workflow::new()
        .with_id(&mid)
        .with_step(|step| step.with_act(Act::req(|act| act.with_id("test"))));

    engine.manager().deploy(&workflow).unwrap();
    let mut options = Vars::new();
    options.insert("pid".to_string(), "".into());
    let result = executor.start(&workflow.id, &options);
    assert_eq!(result.is_ok(), true);
}

#[tokio::test]
async fn export_executor_start_dup_pid_error() {
    let engine = Engine::new();
    let executor = engine.executor();

    let pid = utils::longid();
    let mid = utils::longid();
    let model = Workflow::new()
        .with_id(&mid)
        .with_step(|step| step.with_act(Act::req(|act| act.with_id("test"))));

    let store = engine.runtime().cache().store();
    let proc = data::Proc {
        id: pid.clone(),
        name: model.name.clone(),
        mid: model.id.clone(),
        state: TaskState::None.to_string(),
        start_time: 0,
        end_time: 0,
        timestamp: 0,
        model: model.to_json().unwrap(),
        env_local: "{}".to_string(),
        err: None,
    };
    store.procs().create(&proc).expect("create proc");
    engine
        .manager()
        .deploy(&model)
        .expect("fail to deploy workflow");
    let mut options = Vars::new();
    options.insert("pid".to_string(), json!(pid.to_string()));
    let result = executor.start(&model.id, &options);
    assert_eq!(result.is_err(), true);
}

#[tokio::test]
async fn export_manager_models_get() {
    let engine = Engine::new();
    let manager = engine.manager();
    let mut model = Workflow::new().with_step(|step| step.with_id("step1"));

    for _ in 0..5 {
        model.set_id(&utils::longid());
        manager.deploy(&model).unwrap();
    }

    let result = manager.models(10).unwrap();
    assert_eq!(result.len(), 5);
}

#[tokio::test]
async fn export_manager_model_get_text() {
    let engine = Engine::new();
    let manager = engine.manager();
    let mut model = Workflow::new().with_step(|step| step.with_id("step1"));

    model.set_id(&utils::longid());
    manager.deploy(&model).unwrap();

    let result = manager.model(&model.id, "text").unwrap();
    assert_eq!(result.id, model.id);
    assert_eq!(result.model.is_empty(), false);
}

#[tokio::test]
async fn export_manager_model_get_tree() {
    let engine = Engine::new();
    let manager = engine.manager();
    let mut model = Workflow::new().with_step(|step| step.with_id("step1"));

    model.set_id(&utils::longid());
    manager.deploy(&model).unwrap();

    let result = manager.model(&model.id, "tree").unwrap();
    assert_eq!(result.id, model.id);
    assert_eq!(result.model.is_empty(), false);
}

#[tokio::test]
async fn export_manager_model_remove() {
    let engine = Engine::new();
    let manager = engine.manager();
    let mut model = Workflow::new().with_step(|step| step.with_id("step1"));

    model.set_id(&utils::longid());
    manager.deploy(&model).unwrap();

    manager.remove(&model.id).unwrap();
    assert_eq!(manager.models(10).unwrap().len(), 0);
}

#[tokio::test]
async fn export_manager_procs_get_one() {
    let engine = Engine::new();
    let manager = engine.manager();
    let model = Workflow::new().with_step(|step| {
        step.with_id("step1")
            .with_act(Act::req(|act| act.with_id("act1")))
    });

    let rt = engine.runtime();
    let sig = engine.signal(());
    let s1 = sig.clone();
    let proc = rt.create_proc(&utils::longid(), &model);
    engine.channel().on_start(move |_| s1.close());
    rt.launch(&proc);
    sig.recv().await;

    assert_eq!(manager.procs(10).unwrap().len(), 1);
}

#[tokio::test]
async fn export_manager_procs_get_many() {
    let engine = Engine::new();
    let manager = engine.manager();
    let model = Workflow::new().with_step(|step| {
        step.with_id("step1")
            .with_act(Act::req(|act| act.with_id("act1")))
    });

    let rt = engine.runtime();
    let sig = engine.signal(());
    let s1 = sig.clone();
    let count = Arc::new(Mutex::new(0));
    engine.channel().on_start(move |_e| {
        println!("message:{_e:?}");
        let mut count = count.lock().unwrap();
        *count += 1;

        if *count == 5 {
            s1.close();
        }
    });
    for _ in 0..5 {
        let proc = rt.create_proc(&utils::longid(), &model);
        rt.launch(&proc);
    }
    sig.recv().await;
    assert_eq!(manager.procs(10).unwrap().len(), 5);
}

#[tokio::test]
async fn export_manager_proc_get_json() {
    let engine = Engine::new();
    let manager = engine.manager();
    let model = Workflow::new().with_step(|step| {
        step.with_id("step1")
            .with_act(Act::req(|act| act.with_id("act1")))
    });

    let rt = engine.runtime();
    let sig = engine.signal(());
    let s1 = sig.clone();
    engine.channel().on_start(move |_| s1.close());
    let pid = utils::longid();
    let proc = rt.create_proc(&pid, &model);
    rt.launch(&proc);
    sig.recv().await;

    let info = manager.proc(&pid, "json").unwrap();
    assert_eq!(info.id, pid);
    assert_eq!(info.tasks.is_empty(), false);
}

#[tokio::test]
async fn export_manager_proc_get_tree() {
    let engine = Engine::new();
    let manager = engine.manager();
    let model = Workflow::new().with_step(|step| {
        step.with_id("step1")
            .with_act(Act::req(|act| act.with_id("act1")))
    });

    let rt = engine.runtime();
    let sig = engine.signal(());
    let s1 = sig.clone();
    engine.channel().on_start(move |_| s1.close());
    let pid = utils::longid();
    let proc = rt.create_proc(&pid, &model);
    rt.launch(&proc);
    sig.recv().await;

    let info = manager.proc(&pid, "tree").unwrap();
    assert_eq!(info.id, pid);
    assert_eq!(info.tasks.is_empty(), false);
}

#[tokio::test]
async fn export_manager_tasks() {
    let engine = Engine::new();
    let manager = engine.manager();
    let model = Workflow::new().with_step(|step| {
        step.with_id("step1")
            .with_act(Act::req(|act| act.with_id("act1")))
    });

    let rt = engine.runtime();
    let sig = engine.signal(());
    let s1 = sig.clone();
    engine.channel().on_message(move |e| {
        if e.is_key("act1") {
            s1.close()
        }
    });
    let pid = utils::longid();
    let mut vars = Vars::new();
    vars.insert("uid".to_string(), json!("u1"));
    vars.insert("pid".to_string(), json!(pid));

    rt.start(&model, &vars).unwrap();
    sig.recv().await;

    let tasks = manager.tasks(&pid, 10).unwrap();
    assert_eq!(tasks.len(), 3); // 3 means the tasks with workflow step act
}

#[tokio::test]
async fn export_manager_task_get() {
    let engine = Engine::new();
    let manager = engine.manager();
    let model = Workflow::new().with_step(|step| {
        step.with_id("step1")
            .with_act(Act::req(|act| act.with_id("act1")))
    });

    let rt = engine.runtime();
    let sig = engine.signal(());
    let s1 = sig.clone();
    engine.channel().on_message(move |e| {
        if e.is_key("act1") {
            s1.close()
        }
    });
    let pid = utils::longid();
    let mut vars = Vars::new();
    vars.insert("uid".to_string(), json!("u1"));
    vars.insert("pid".to_string(), json!(pid));

    rt.start(&model, &vars).unwrap();
    sig.recv().await;
    let tasks = manager.tasks(&pid, 10).unwrap();
    let mut result = true;
    for task in tasks {
        result &= manager.task(&pid, &task.id).is_ok();
    }
    assert_eq!(result, true);
}

#[tokio::test]
async fn export_executeor_start() {
    let engine = Engine::new();
    let model = Workflow::new()
        .with_id(&utils::longid())
        .with_step(|step| step.with_id("step1"));

    let sig = engine.signal(());
    let s1 = sig.clone();
    engine.channel().on_complete(move |_| s1.close());

    engine.manager().deploy(&model).unwrap();

    let pid = utils::longid();
    let mut vars = Vars::new();
    vars.insert("uid".to_string(), json!("u1"));
    vars.insert("pid".to_string(), json!(pid));

    let result = engine.executor().start(&model.id, &vars);
    sig.recv().await;
    assert_eq!(result.is_ok(), true);
}

#[tokio::test]
async fn export_executeor_start_not_found_model() {
    let engine = Engine::new();
    let sig = engine.signal(());
    let s1 = sig.clone();
    engine.channel().on_complete(move |_| s1.close());

    let pid = utils::longid();
    let mut vars = Vars::new();
    vars.insert("uid".to_string(), json!("u1"));
    vars.insert("pid".to_string(), json!(pid));

    let result = engine.executor().start("not_exists", &vars);
    assert_eq!(result.is_ok(), false);
}

#[tokio::test]
async fn export_executeor_complete() {
    let engine = Engine::new();
    let model = Workflow::new().with_step(|step| {
        step.with_id("step1")
            .with_act(Act::req(|act| act.with_id("act1")))
    });

    let rt = engine.runtime();
    let sig = engine.signal(false);
    let s1 = sig.clone();
    engine.channel().on_message(move |e| {
        if e.is_key("act1") && e.is_state("created") {
            let mut vars = Vars::new();
            vars.insert("uid".to_string(), json!("u1"));
            let ret = engine.executor().complete(&e.pid, &e.tid, &vars);
            s1.send(ret.is_ok());
        }
    });
    let mut vars = Vars::new();
    vars.insert("uid".to_string(), json!("u1"));
    rt.start(&model, &vars).unwrap();
    let ret = sig.recv().await;
    assert_eq!(ret, true);
}

#[tokio::test]
async fn export_executeor_complete_no_uid() {
    let engine = Engine::new();
    let model = Workflow::new().with_step(|step| {
        step.with_id("step1")
            .with_act(Act::req(|act| act.with_id("act1")))
    });

    let rt = engine.runtime();
    let sig = engine.signal(false);
    let s1 = sig.clone();
    // scher.emitter().on_complete(|e| rx.close());

    engine.channel().on_message(move |e| {
        if e.is_key("act1") && e.is_state("created") {
            let vars = Vars::new();
            let ret = engine.executor().complete(&e.pid, &e.tid, &vars);

            // no uid is still ok in version 0.7.0+
            s1.send(ret.is_ok());
        }
    });
    rt.start(&model, &Vars::new()).unwrap();
    let ret = sig.recv().await;
    assert_eq!(ret, true);
}

#[tokio::test]
async fn export_executeor_submit() {
    let engine = Engine::new();
    let model = Workflow::new().with_step(|step| {
        step.with_id("step1")
            .with_act(Act::req(|act| act.with_id("act1")))
    });

    let rt = engine.runtime();
    let sig = engine.signal(false);
    let s1 = sig.clone();
    // scher.emitter().on_complete(|e| e.close());

    engine.channel().on_message(move |e| {
        if e.is_key("act1") && e.is_state("created") {
            let mut vars = Vars::new();
            vars.insert("uid".to_string(), json!("u1"));
            let ret = engine.executor().submit(&e.pid, &e.tid, &vars);
            s1.send(ret.is_ok());
        }
    });
    let mut vars = Vars::new();
    vars.insert("uid".to_string(), json!("u1"));
    rt.start(&model, &vars).unwrap();
    let ret = sig.recv().await;
    assert_eq!(ret, true);
}

#[tokio::test]
async fn export_executeor_skip() {
    let engine = Engine::new();
    let model = Workflow::new().with_step(|step| {
        step.with_id("step1")
            .with_act(Act::req(|act| act.with_id("act1")))
    });

    let rt = engine.runtime();
    let sig = engine.signal(false);
    let s1 = sig.clone();
    // scher.emitter().on_complete(|e| e.close());
    engine.channel().on_message(move |e| {
        if e.is_key("act1") && e.is_state("created") {
            let mut vars = Vars::new();
            vars.insert("uid".to_string(), json!("u1"));
            let ret = engine.executor().skip(&e.pid, &e.tid, &vars);
            s1.send(ret.is_ok());
        }
    });
    let mut vars = Vars::new();
    vars.insert("uid".to_string(), json!("u1"));
    rt.start(&model, &vars).unwrap();
    let ret = sig.recv().await;
    assert_eq!(ret, true);
}

#[tokio::test]
async fn export_executeor_error() {
    let engine = Engine::new();
    let model = Workflow::new().with_step(|step| {
        step.with_id("step1")
            .with_act(Act::req(|act| act.with_id("act1")))
    });

    let rt = engine.runtime();
    let sig = engine.signal(false);
    let s1 = sig.clone();
    engine.channel().on_message(move |e| {
        if e.is_key("act1") && e.is_state("created") {
            let mut vars = Vars::new();
            vars.insert("uid".to_string(), json!("u1"));
            vars.insert("error".to_string(), json!({ "ecode": "code_1"}));
            let ret = engine.executor().error(&e.pid, &e.tid, &vars);
            s1.send(ret.is_ok());
        }
    });
    let mut vars = Vars::new();
    vars.insert("uid".to_string(), json!("u1"));
    rt.start(&model, &vars).unwrap();
    let ret = sig.recv().await;
    assert_eq!(ret, true);
}

#[tokio::test]
async fn export_executeor_abort() {
    let engine = Engine::new();
    let model = Workflow::new().with_step(|step| {
        step.with_id("step1")
            .with_act(Act::req(|act| act.with_id("act1")))
    });

    let rt = engine.runtime();
    let sig = engine.signal(false);
    let s1 = sig.clone();
    // scher.emitter().on_complete(|e| e.close());
    engine.channel().on_message(move |e| {
        println!("message: {:?}", e.inner());
        if e.is_key("act1") && e.is_state("created") {
            let mut vars = Vars::new();
            vars.insert("uid".to_string(), json!("u1"));
            let ret = engine.executor().abort(&e.pid, &e.tid, &vars);
            s1.send(ret.is_ok());
        }
    });
    let mut vars = Vars::new();
    vars.insert("uid".to_string(), json!("u1"));
    rt.start(&model, &vars).unwrap();
    let ret = sig.recv().await;
    assert_eq!(ret, true);
}

#[tokio::test]
async fn export_executeor_back() {
    let engine = Engine::new();
    let model = Workflow::new()
        .with_step(|step| {
            step.with_id("step1")
                .with_act(Act::req(|act| act.with_id("act1")))
        })
        .with_step(|step| {
            step.with_id("step2")
                .with_act(Act::req(|act| act.with_id("act2")))
        });

    let rt = engine.runtime();
    let sig = engine.signal(false);
    let s1 = sig.clone();
    // scher.emitter().on_complete(|e| e.close());

    let count = Arc::new(Mutex::new(0));
    engine.channel().on_message(move |e| {
        if e.is_key("act1") && e.is_state("created") {
            let mut count = count.lock().unwrap();
            if *count == 1 {
                s1.close();
            }
            let mut vars = Vars::new();
            vars.insert("uid".to_string(), json!("u1"));
            engine.executor().complete(&e.pid, &e.tid, &vars).unwrap();

            *count += 1;
        }

        if e.is_key("act2") && e.is_state("created") {
            let mut vars = Vars::new();
            vars.insert("uid".to_string(), json!("u1"));
            vars.insert("to".to_string(), json!("step1"));
            let ret = engine.executor().back(&e.pid, &e.tid, &vars);
            s1.update(|data| *data = ret.is_ok());
        }
    });
    let mut vars = Vars::new();
    vars.insert("uid".to_string(), json!("u1"));
    rt.start(&model, &vars).unwrap();
    let ret = sig.recv().await;
    assert_eq!(ret, true);
}

#[tokio::test]
async fn export_executeor_cancel() {
    let engine = Engine::new();
    let model = Workflow::new()
        .with_step(|step| {
            step.with_id("step1")
                .with_act(Act::req(|act| act.with_id("act1")))
        })
        .with_step(|step| {
            step.with_id("step2")
                .with_act(Act::req(|act| act.with_id("act2")))
        });

    let rt = engine.runtime();
    let sig = engine.signal(false);
    let s1 = sig.clone();
    // scher.emitter().on_complete(|e| e.close());
    let count = Arc::new(Mutex::new(0));
    let tid = Arc::new(Mutex::new("".to_string()));
    engine.channel().on_message(move |e| {
        if e.is_key("act1") && e.is_state("created") {
            let mut count = count.lock().unwrap();
            if *count == 1 {
                s1.close();
            }
            let mut vars = Vars::new();
            vars.insert("uid".to_string(), json!("u1"));
            engine.executor().complete(&e.pid, &e.tid, &vars).unwrap();

            *tid.lock().unwrap() = e.tid.clone();
            *count += 1;
        }

        if e.is_key("act2") && e.is_state("created") {
            let mut vars = Vars::new();
            vars.insert("uid".to_string(), json!("u1"));
            let ret = engine
                .executor()
                .cancel(&e.pid, &tid.lock().unwrap(), &vars);
            s1.update(|data| *data = ret.is_ok());
        }
    });
    let mut vars = Vars::new();
    vars.insert("uid".to_string(), json!("u1"));
    rt.start(&model, &vars).unwrap();
    let ret = sig.recv().await;
    assert_eq!(ret, true);
}

#[tokio::test]
async fn export_executeor_push() {
    let engine = Engine::new();
    let model = Workflow::new().with_step(|step| {
        step.with_id("step1")
            .with_act(Act::req(|act| act.with_id("act1")))
    });

    let rt = engine.runtime();
    let sig = engine.signal(false);
    let s1 = sig.clone();
    // scher.emitter().on_complete(|e| e.close());
    engine.channel().on_message(move |e| {
        println!("message: {e:?}");
        if e.is_key("step1") && e.is_state("created") {
            let mut vars = Vars::new();
            vars.insert("id".to_string(), json!("act2"));
            engine.executor().push(&e.pid, &e.tid, &vars).unwrap();
        }

        if e.is_key("act2") && e.is_state("created") {
            s1.send(true);
        }
    });
    let mut vars = Vars::new();
    vars.insert("uid".to_string(), json!("u1"));
    rt.start(&model, &vars).unwrap();
    let ret = sig.recv().await;
    assert_eq!(ret, true);
}

#[tokio::test]
async fn export_executeor_push_no_id_error() {
    let engine = Engine::new();
    let model = Workflow::new().with_step(|step| {
        step.with_id("step1")
            .with_act(Act::req(|act| act.with_id("act1")))
    });

    let rt = engine.runtime();
    let sig = engine.signal(false);
    let s1 = sig.clone();
    // scher.emitter().on_complete(|e| e.close());
    engine.channel().on_message(move |e| {
        println!("message: {e:?}");
        if e.is_key("step1") && e.is_state("created") {
            s1.send(
                engine
                    .executor()
                    .push(&e.pid, &e.tid, &Vars::new())
                    .is_err(),
            );
        }
    });
    let mut vars = Vars::new();
    vars.insert("uid".to_string(), json!("u1"));
    rt.start(&model, &vars).unwrap();
    let ret = sig.recv().await;
    assert_eq!(ret, true);
}

#[tokio::test]
async fn export_executeor_push_not_step_id_error() {
    let engine = Engine::new();
    let model = Workflow::new().with_step(|step| {
        step.with_id("step1")
            .with_act(Act::req(|act| act.with_id("act1")))
    });

    let rt = engine.runtime();
    let sig = engine.signal(false);
    let s1 = sig.clone();
    // scher.emitter().on_complete(|e| e.close());
    engine.channel().on_message(move |e| {
        println!("message: {e:?}");
        if e.is_key("act1") && e.is_state("created") {
            let vars = Vars::new();
            s1.send(engine.executor().push(&e.pid, &e.tid, &vars).is_err());
        }
    });
    let mut vars = Vars::new();
    vars.insert("uid".to_string(), json!("u1"));
    rt.start(&model, &vars).unwrap();
    let ret = sig.recv().await;
    assert_eq!(ret, true);
}

#[tokio::test]
async fn export_executeor_remove() {
    let engine = Engine::new();
    let model = Workflow::new().with_step(|step| {
        step.with_id("step1")
            .with_act(Act::req(|act| act.with_id("act1")))
    });

    let rt = engine.runtime();
    let sig = engine.signal(false);
    let s1 = sig.clone();
    engine.channel().on_message(move |e| {
        println!("message: {e:?}");
        if e.is_key("act1") && e.is_state("created") {
            s1.send(
                engine
                    .executor()
                    .remove(&e.pid, &e.tid, &Vars::new())
                    .is_ok(),
            );
        }
    });
    let mut vars = Vars::new();
    vars.insert("uid".to_string(), json!("u1"));
    rt.start(&model, &vars).unwrap();
    let ret = sig.recv().await;
    assert_eq!(ret, true);
}

#[tokio::test]
async fn engine_extender_register_plugin() {
    let engine = Engine::new();
    let plugin_count = engine.plugins().lock().unwrap().len();
    let extender = engine.extender();
    extender.register_plugin(&TestPlugin::default());
    assert_eq!(engine.plugins().lock().unwrap().len(), plugin_count + 1);
}

#[tokio::test]
async fn export_extender_register_module() {
    let engine = Engine::new();
    let extender = engine.extender();

    let before_count = engine.runtime().env().modules_count();
    let module = test_module::TestModule;
    extender.register_module(&module);
    let count = engine.runtime().env().modules_count();
    assert_eq!(count, before_count + 1);
}

#[tokio::test]
async fn export_emitter_default() {
    let engine = Engine::new();
    let emitter = engine.channel();
    let sig = engine.signal::<Vec<Message>>(Vec::new());
    let s = sig.clone();
    emitter.on_message(move |e| {
        s.update(|data| data.push(e.inner().clone()));
        s.close();
    });

    let msg = Message::default();
    engine.runtime().emitter().emit_message(&msg);
    let ret = sig.recv().await;
    assert_eq!(ret.len(), 1);
}

#[tokio::test]
async fn export_emitter_type_match() {
    let engine = Engine::new();
    let emitter = engine.channel_with_options(&ChannelOptions {
        r#type: "a*".to_string(),
        ..Default::default()
    });
    let sig = engine.signal::<Vec<Message>>(Vec::new());
    let s = sig.clone();
    emitter.on_message(move |e| {
        s.update(|data| data.push(e.inner().clone()));
        s.close();
    });

    let msg = Message {
        r#type: "abc".to_string(),
        ..Message::default()
    };
    engine.runtime().emitter().emit_message(&msg);
    let ret = sig.recv().await;
    assert_eq!(ret.len(), 1);
}

#[tokio::test]
async fn export_emitter_type_not_match() {
    let engine = Engine::new();
    let emitter = engine.channel_with_options(&ChannelOptions {
        r#type: "a*".to_string(),
        ..Default::default()
    });
    let sig = engine.signal::<Vec<Message>>(Vec::new());
    let s = sig.clone();
    emitter.on_message(move |e| {
        s.update(|data| data.push(e.inner().clone()));
        s.close();
    });

    let msg = Message {
        r#type: "bac".to_string(),
        ..Message::default()
    };
    engine.runtime().emitter().emit_message(&msg);
    let ret = sig.timeout(100).await;
    assert_eq!(ret.len(), 0);
}

#[tokio::test]
async fn export_emitter_state_match() {
    let engine = Engine::new();
    let emitter = engine.channel_with_options(&ChannelOptions {
        state: "completed".to_string(),
        ..Default::default()
    });
    let sig = engine.signal::<Vec<Message>>(Vec::new());
    let s = sig.clone();
    emitter.on_message(move |e| {
        s.update(|data| data.push(e.inner().clone()));
        s.close();
    });

    let msg = Message {
        state: MessageState::Completed,
        ..Message::default()
    };
    engine.runtime().emitter().emit_message(&msg);
    let ret = sig.recv().await;
    assert_eq!(ret.len(), 1);
}

#[tokio::test]
async fn export_emitter_state_not_match() {
    let engine = Engine::new();
    let emitter = engine.channel_with_options(&ChannelOptions {
        r#type: "error".to_string(),
        ..Default::default()
    });
    let sig = engine.signal::<Vec<Message>>(Vec::new());
    let s = sig.clone();
    emitter.on_message(move |e| {
        s.update(|data| data.push(e.inner().clone()));
        s.close();
    });

    let msg = Message {
        state: MessageState::Completed,
        ..Message::default()
    };
    engine.runtime().emitter().emit_message(&msg);
    let ret = sig.timeout(100).await;
    assert_eq!(ret.len(), 0);
}

#[tokio::test]
async fn export_emitter_tag_match() {
    let engine = Engine::new();
    let emitter = engine.channel_with_options(&ChannelOptions {
        tag: "tag*".to_string(),
        ..Default::default()
    });
    let sig = engine.signal::<Vec<Message>>(Vec::new());
    let s = sig.clone();
    emitter.on_message(move |e| {
        s.update(|data| data.push(e.inner().clone()));
    });

    let msg = Message {
        tag: "tag1".to_string(),
        ..Message::default()
    };
    engine.runtime().emitter().emit_message(&msg);

    let msg = Message {
        tag: "aaaa".to_string(),
        model: Model {
            tag: "tag2".to_string(),
            ..Default::default()
        },
        ..Message::default()
    };
    engine.runtime().emitter().emit_message(&msg);

    let ret = sig.timeout(100).await;
    assert_eq!(ret.len(), 2);
}

#[tokio::test]
async fn export_emitter_tag_not_match() {
    let engine = Engine::new();
    let emitter = engine.channel_with_options(&ChannelOptions {
        tag: "tag*".to_string(),
        ..Default::default()
    });
    let sig = engine.signal::<Vec<Message>>(Vec::new());
    let s = sig.clone();
    emitter.on_message(move |e| {
        s.update(|data| data.push(e.inner().clone()));
        s.close();
    });

    let msg = Message {
        tag: "aaaa".to_string(),
        ..Message::default()
    };
    engine.runtime().emitter().emit_message(&msg);
    let ret = sig.timeout(100).await;
    assert_eq!(ret.len(), 0);
}

#[tokio::test]
async fn export_emitter_key_match() {
    let engine = Engine::new();
    let emitter = engine.channel_with_options(&ChannelOptions {
        key: "key*".to_string(),
        ..Default::default()
    });
    let sig = engine.signal::<Vec<Message>>(Vec::new());
    let s = sig.clone();
    emitter.on_message(move |e| {
        s.update(|data| data.push(e.inner().clone()));
        s.close();
    });

    let msg = Message {
        key: "key1".to_string(),
        ..Message::default()
    };
    engine.runtime().emitter().emit_message(&msg);
    let ret = sig.recv().await;
    assert_eq!(ret.len(), 1);
}

#[tokio::test]
async fn export_emitter_key_not_match() {
    let engine = Engine::new();
    let emitter = engine.channel_with_options(&ChannelOptions {
        key: "key*".to_string(),
        ..Default::default()
    });
    let sig = engine.signal::<Vec<Message>>(Vec::new());
    let s = sig.clone();
    emitter.on_message(move |e| {
        s.update(|data| data.push(e.inner().clone()));
        s.close();
    });

    let msg = Message {
        key: "aaaa".to_string(),
        ..Message::default()
    };
    engine.runtime().emitter().emit_message(&msg);
    let ret = sig.timeout(100).await;
    assert_eq!(ret.len(), 0);
}

#[tokio::test]
async fn export_message_store_with_emit_id() {
    let engine = Engine::new();
    let emitter = engine.channel_with_options(&ChannelOptions {
        id: "my_emit_id".to_string(),
        ack: true,
        ..Default::default()
    });
    let (s1, s2) = engine.signal::<Message>(Message::default()).double();
    emitter.on_message(move |e| {
        s1.send(e.inner().clone());
    });

    let msg = Message {
        id: "1".to_string(),
        ..Message::default()
    };
    engine.runtime().emitter().emit_message(&msg);
    let ret = s2.recv().await;
    assert_eq!(ret.id, "1");
    assert_eq!(
        engine
            .runtime()
            .cache()
            .store()
            .messages()
            .exists("1")
            .unwrap(),
        true
    );
}

#[tokio::test]
async fn export_message_store_with_emit_id_and_options() {
    let engine = Engine::new();
    let emitter = engine.channel_with_options(&ChannelOptions {
        id: "my_emit_id".to_string(),
        tag: "tag*".to_string(),
        ack: true,
        ..Default::default()
    });
    let (s1, s2) = engine.signal::<Message>(Message::default()).double();
    emitter.on_message(move |e| {
        s1.send(e.inner().clone());
    });

    let msg = Message {
        id: utils::longid(),
        tag: "tagaaaa".to_string(),
        ..Message::default()
    };
    engine.runtime().emitter().emit_message(&msg);
    let ret = s2.recv().await;
    assert_eq!(ret.id, msg.id);
    let message = engine
        .runtime()
        .cache()
        .store()
        .messages()
        .find(&msg.id)
        .unwrap();
    assert_eq!(message.tag, msg.tag);
    assert_eq!(message.chan_id, "my_emit_id");
    assert_eq!(message.chan_pattern, "*:*:tag*:*");
}

#[tokio::test]
async fn export_message_not_store_without_emit_id() {
    let engine = Engine::new();
    let emitter = engine.channel_with_options(&ChannelOptions {
        id: "my_emit_id".to_string(),
        tag: "tag*".to_string(),
        ..Default::default()
    });
    let (s1, s2) = engine.signal::<Message>(Message::default()).double();
    emitter.on_message(move |e| {
        s1.send(e.inner().clone());
    });

    let msg = Message {
        id: utils::longid(),
        tag: "not_match_tag".to_string(),
        ..Message::default()
    };
    engine.runtime().emitter().emit_message(&msg);
    s2.timeout(20).await;
    assert_eq!(
        engine
            .runtime()
            .cache()
            .store()
            .messages()
            .exists(&msg.id)
            .unwrap(),
        false
    );
}

#[tokio::test]
async fn export_message_not_store_with_empty_emit_id_and_not_match_option() {
    let engine = Engine::new();
    let emitter = engine.channel_with_options(&ChannelOptions {
        id: "".to_string(),
        ..Default::default()
    });
    let (s1, s2) = engine.signal::<Message>(Message::default()).double();
    emitter.on_message(move |e| {
        s1.send(e.inner().clone());
    });

    let msg = Message {
        id: "1".to_string(),
        ..Message::default()
    };
    engine.runtime().emitter().emit_message(&msg);
    s2.timeout(20).await;
    assert_eq!(
        engine
            .runtime()
            .cache()
            .store()
            .messages()
            .exists("1")
            .unwrap(),
        false
    );
}

#[tokio::test]
async fn export_message_clear_error_messages() {
    let engine = Engine::new();
    let msg = data::Message {
        id: utils::longid(),
        status: data::MessageStatus::Error,
        ..data::Message::default()
    };
    engine
        .runtime()
        .cache()
        .store()
        .messages()
        .create(&msg)
        .unwrap();
    let message = engine
        .runtime()
        .cache()
        .store()
        .messages()
        .find(&msg.id)
        .unwrap();
    assert_eq!(message.status, data::MessageStatus::Error);
    engine.manager().clear_error_messages().unwrap();
    assert_eq!(
        engine
            .runtime()
            .cache()
            .store()
            .messages()
            .exists(&msg.id)
            .unwrap(),
        false
    );
}

#[tokio::test]
async fn export_message_resend_error_messages() {
    let engine = Engine::new();
    let msg = data::Message {
        id: utils::longid(),
        status: data::MessageStatus::Error,
        ..data::Message::default()
    };
    engine
        .runtime()
        .cache()
        .store()
        .messages()
        .create(&msg)
        .unwrap();
    let message = engine
        .runtime()
        .cache()
        .store()
        .messages()
        .find(&msg.id)
        .unwrap();
    assert_eq!(message.status, data::MessageStatus::Error);
    engine.manager().resend_error_messages().unwrap();

    let message = engine
        .runtime()
        .cache()
        .store()
        .messages()
        .find(&msg.id)
        .unwrap();
    assert_eq!(message.status, data::MessageStatus::Created);
    assert_eq!(message.retry_times, 0);
}

#[derive(Debug, Default, Clone)]
struct TestPlugin;

impl ActPlugin for TestPlugin {
    fn on_init(&self, _engine: &Engine) {
        println!("TestPlugin");
    }
}

mod test_module {
    use crate::ActModule;

    #[derive(Clone)]
    pub struct TestModule;
    impl ActModule for TestModule {
        fn init<'a>(&self, _ctx: &rquickjs::Ctx<'a>) -> crate::Result<()> {
            Ok(())
        }
    }
}
