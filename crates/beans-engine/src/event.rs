use std::{
    fs::File,
    io::{BufReader, Read},
    sync::{Arc, Mutex},
    thread,
};

use beans_core::{
    events::{Event, ResourceProcessed},
    resource::{ResourceId, ResourceRoot},
    revision::Revision,
};
use crossbeam_channel::{Receiver, Sender, select_biased, unbounded};

use crate::{Engine, storage::Storage};

impl Engine {
    pub fn start(&mut self) -> Result<(), String> {
        if self.worker.is_some() {
            return Err("Engine is already started".into());
        }
        let storage = Storage::new(&self.storage.path)?;
        let events = self.receiver.clone();
        let stopping = self.stopping.clone();
        let listeners = self.listeners.clone();
        self.worker = Some(
            thread::Builder::new()
                .name("beans-events".into())
                .spawn(move || run(storage, events, stopping, listeners))
                .map_err(|error| error.to_string())?,
        );
        Ok(())
    }

    /// Subscribes to future received events and processing completions.
    /// Each listener has an unbounded queue and is responsible for draining it.
    pub fn subscribe(&self) -> Receiver<Event> {
        let (sender, receiver) = unbounded();
        self.listeners
            .lock()
            .expect("event listener registry poisoned")
            .push(sender);
        receiver
    }
}

impl Drop for Engine {
    fn drop(&mut self) {
        if let Some(worker) = self.worker.take() {
            let _ = self.shutdown.send(());
            if worker.join().is_err() {
                log::error!("Engine event thread panicked");
            }
        }
    }
}

fn run(
    mut storage: Storage,
    events: Receiver<Event>,
    stopping: Receiver<()>,
    listeners: Arc<Mutex<Vec<Sender<Event>>>>,
) {
    loop {
        select_biased! {
            recv(stopping) -> _ => break,
            recv(events) -> event => {
                let Ok(event) = event else { break; };
                publish(&listeners, &event);
                if let Event::ResourceFound(found) = event {
                    let result = process_resource(&mut storage, &found.resource, &stopping);
                    if !stopping.is_empty() {
                        break;
                    }
                    if let Err(error) = &result {
                        log::warn!("{:?}: {error}", found.resource);
                    }
                    publish(&listeners, &Event::ResourceProcessed(ResourceProcessed {
                        resource: found.resource,
                        result,
                    }));
                }
            }
        }
    }
}

fn publish(listeners: &Mutex<Vec<Sender<Event>>>, event: &Event) {
    listeners
        .lock()
        .expect("event listener registry poisoned")
        .retain(|listener| listener.send(event.clone()).is_ok());
}

fn process_resource(
    storage: &mut Storage,
    resource: &ResourceId,
    stopping: &Receiver<()>,
) -> Result<Option<Revision>, String> {
    if !stopping.is_empty() {
        return Ok(None);
    }
    let ResourceRoot::File { path } = &resource.root else {
        return Err("cannot open a virtual resource from discovery".into());
    };
    if !resource.entries.is_empty() {
        return Err("opening nested resources directly is unsupported".into());
    }
    if !std::fs::metadata(path)
        .map_err(|error| error.to_string())?
        .is_file()
    {
        return Err("expected a resource file".into());
    }
    let mut reader = BufReader::new(File::open(path).map_err(|error| error.to_string())?);
    if let Some(classes) = beans_platform_jvm::process(resource, &mut reader)? {
        let mut models = Vec::new();
        for class in classes {
            if !stopping.is_empty() {
                return Ok(None);
            }
            let (id, class) = class?;
            models.push((id.to_url()?, beans_platform_jvm::codec::encode(&class)?));
        }
        return if stopping.is_empty() {
            storage.publish_jvm(&models)
        } else {
            Ok(None)
        };
    }
    if !beans_lang_java::accept(path) {
        return Ok(None);
    }
    let mut contents = String::new();
    reader
        .read_to_string(&mut contents)
        .map_err(|error| error.to_string())?;
    if stopping.is_empty()
        && let Some(file) = beans_lang_java::process(resource, &contents)
    {
        return storage.publish_java(resource, &file, &contents).map(Some);
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::ModelQuery;
    use beans_core::{
        events::{JavaHomeFound, ResourceFound},
        resource::ResourceEntry,
    };
    use beans_lang_java::model_query::JavaModelQuery;
    use beans_platform_jvm::model_query::JvmModelQuery;
    use beans_testing::support::class_bytes;
    use rusqlite::{Connection, OpenFlags};
    use std::{
        io::{Cursor, Write},
        path::Path,
        time::{Duration, Instant},
    };
    use tempfile::tempdir;
    use zip::{ZipWriter, write::SimpleFileOptions};

    #[test]
    fn subscribers_receive_independent_streams_without_consuming_work() {
        let root = tempdir().unwrap();
        let path = root.path().join("index.sqlite");
        let mut engine = Engine::new(&path).unwrap();
        drop(engine.subscribe());
        let first = engine.subscribe();
        engine.start().unwrap();
        let second = engine.subscribe();
        let source = ResourceId::file(root.path().join("Example.java"));
        std::fs::write(root.path().join("Example.java"), "class Example {}").unwrap();
        send(&engine.sender(), source.clone());
        let Event::ResourceFound(found) = first.recv_timeout(Duration::from_secs(5)).unwrap()
        else {
            panic!("expected to observe ResourceFound");
        };
        assert_eq!(found.resource, source);
        let completed = processed(&first, &source);
        assert_eq!(completed.result.unwrap(), Some(Revision::new(1)));
        let mut reader = reader(&path);
        let query = ModelQuery::new(&mut reader).unwrap();
        assert!(query.java_file(&source).unwrap().is_some());
        assert_eq!(query.revision(), Revision::new(1));
        let Event::ResourceFound(found) = second.recv_timeout(Duration::from_secs(5)).unwrap()
        else {
            panic!("a slow listener lost ResourceFound");
        };
        assert_eq!(found.resource, source);
        assert_eq!(
            processed(&second, &source).result.unwrap(),
            Some(Revision::new(1))
        );
        assert_eq!(engine.listeners.lock().unwrap().len(), 2);
    }

    #[test]
    fn completions_report_failures_and_no_publication_without_stopping_the_worker() {
        let root = tempdir().unwrap();
        let path = root.path().join("index.sqlite");
        let mut engine = Engine::new(&path).unwrap();
        let events = engine.subscribe();
        engine.start().unwrap();
        for (name, bytes, failed) in [
            ("unknown", b"unsupported".to_vec(), false),
            ("empty.jar", jar(&[]), false),
            (
                "broken.jar",
                jar(&[
                    ("p/C.class", class_bytes("p/C")),
                    ("bad.class", b"\xCA\xFE\xBA\xBE".to_vec()),
                ]),
                true,
            ),
        ] {
            let source = ResourceId::file(root.path().join(name));
            std::fs::write(root.path().join(name), bytes).unwrap();
            send(&engine.sender(), source.clone());
            let completion = processed(&events, &source);
            if failed {
                assert!(completion.result.is_err());
            } else {
                assert_eq!(completion.result.unwrap(), None);
            }
        }
        let source = ResourceId::file(root.path().join("Example.java"));
        std::fs::write(root.path().join("Example.java"), "class Example {}").unwrap();
        send(&engine.sender(), source.clone());
        assert_eq!(
            processed(&events, &source).result.unwrap(),
            Some(Revision::new(1))
        );
        let reader = reader(&path);
        assert_eq!(
            reader
                .query_row("SELECT COUNT(*) FROM entries", [], |row| row
                    .get::<_, i64>(0))
                .unwrap(),
            1
        );
    }

    #[test]
    fn processes_a_mise_jdk_and_reports_its_committed_revision() {
        let homes = match beans_testing::support::jdks::list() {
            Ok(homes) => homes,
            Err(error) => {
                eprintln!("skipping mise JDK E2E: {error}");
                return;
            }
        };
        let Some(home) = homes.first() else {
            eprintln!("skipping mise JDK E2E: no installed JDK");
            return;
        };
        let artifact = [
            home.join("lib/modules"),
            home.join("jre/lib/rt.jar"),
            home.join("lib/rt.jar"),
        ]
        .into_iter()
        .find(|path| path.is_file())
        .expect("mise JDK has no supported runtime artifact");
        let root = tempdir().unwrap();
        let path = root.path().join("index.sqlite");
        let mut engine = Engine::new(&path).unwrap();
        let events = engine.subscribe();
        engine.start().unwrap();
        engine
            .sender()
            .send(Event::JavaHomeFound(JavaHomeFound {
                home: home.clone(),
                version: None,
            }))
            .unwrap();
        let resource = ResourceId::file(artifact);
        send(&engine.sender(), resource.clone());
        let completion = processed(&events, &resource);
        assert_eq!(completion.result.unwrap(), Some(Revision::new(1)));
        let mut reader = reader(&path);
        let query = ModelQuery::new(&mut reader).unwrap();
        let object = beans_platform_jvm::find_classes(
            &query,
            beans_core::query_scope::QueryScope::new(
                query.revision(),
                &beans_core::classpath::Unrestricted,
            ),
            "java/lang/Object",
        )
        .unwrap();
        assert_eq!(object.len(), 1);
        assert_eq!(object[0].key.root, resource.root);
        assert_eq!(
            query.jvm_class(&object[0].key).unwrap().unwrap().name,
            "java/lang/Object"
        );
        drop(query);
        drop(engine);
        let reopened = Engine::new(&path).unwrap();
        assert!(reopened.worker.is_none());
        let mut reader = reopened.storage.reader().unwrap();
        let query = ModelQuery::new(&mut reader).unwrap();
        assert_eq!(query.revision(), Revision::new(1));
        assert!(query.jvm_class(&object[0].key).unwrap().is_some());
        eprintln!("indexed mise JDK {}", home.display());
    }

    #[test]
    fn construction_is_inactive_and_start_rejects_a_second_worker() {
        let root = tempdir().unwrap();
        let path = root.path().join("index.sqlite");
        let mut engine = Engine::new(&path).unwrap();
        let source = root.path().join("Example.java");
        std::fs::write(&source, "class Example {}").unwrap();
        send(&engine.sender(), ResourceId::file(source));
        assert!(engine.worker.is_none());
        assert_eq!(engine.receiver.len(), 1);
        let reader = reader(&path);
        assert_eq!(
            reader
                .query_row("SELECT COUNT(*) FROM entries", [], |row| row
                    .get::<_, i64>(0))
                .unwrap(),
            0
        );
        engine.start().unwrap();
        assert_eq!(engine.start().unwrap_err(), "Engine is already started");
        wait_for_rows(&reader, 1);
    }

    #[test]
    fn producers_feed_the_engine_thread_and_java_home_is_ignored() {
        let root = tempdir().unwrap();
        let path = root.path().join("index.sqlite");
        let mut engine = Engine::new(&path).unwrap();
        engine.start().unwrap();
        let first = root.path().join("First.java");
        let second = root.path().join("Second.java");
        std::fs::write(&first, "package p; class First {}").unwrap();
        std::fs::write(&second, "class Second extends Missing {").unwrap();
        engine
            .sender()
            .send(Event::JavaHomeFound(JavaHomeFound {
                home: root.path().join("missing-jdk"),
                version: None,
            }))
            .unwrap();
        send(&engine.sender(), ResourceId::file(&first));
        let sender = engine.sender();
        let second_id = ResourceId::file(&second);
        thread::spawn(move || send(&sender, second_id))
            .join()
            .unwrap();
        let mut reader = reader(&path);
        wait_for_rows(&reader, 2);
        let query = ModelQuery::new(&mut reader).unwrap();
        assert_eq!(
            query
                .java_file(&ResourceId::file(first))
                .unwrap()
                .unwrap()
                .package_name
                .as_slice(),
            ["p"]
        );
        assert!(
            query
                .java_file(&ResourceId::file(second))
                .unwrap()
                .is_some()
        );
        assert_eq!(
            query
                .content(&ResourceId::file(root.path().join("Second.java")))
                .unwrap()
                .unwrap(),
            b"class Second extends Missing {"
        );
        assert_eq!(query.revision().as_i64(), 2);
    }

    #[test]
    fn binary_resources_are_recognized_by_content_and_published_as_one_revision() {
        let class = class_bytes("p/C");
        let jar = jar(&[
            ("p/C.class", class.clone()),
            ("p/D.class", class_bytes("p/D")),
        ]);
        for (bytes, entries) in [
            (class.clone(), vec![None]),
            (jar, vec![Some("p/C.class"), Some("p/D.class")]),
            (jimage(&class), vec![Some("/java.base/p/C.class")]),
        ] {
            let root = tempdir().unwrap();
            let path = root.path().join("index.sqlite");
            let mut engine = Engine::new(&path).unwrap();
            engine.start().unwrap();
            let resource = ResourceId::file(root.path().join("artifact"));
            let ResourceRoot::File { path: artifact } = &resource.root else {
                unreachable!()
            };
            std::fs::write(artifact, bytes).unwrap();
            send(&engine.sender(), resource.clone());
            let mut reader = reader(&path);
            wait_for_rows(&reader, entries.len() as i64);
            let query = ModelQuery::new(&mut reader).unwrap();
            for (index, entry) in entries.iter().enumerate() {
                let id = entry.map_or_else(
                    || resource.clone(),
                    |entry| resource.entry(ResourceEntry(entry.into())),
                );
                assert_eq!(
                    query.jvm_class(&id).unwrap().unwrap().name,
                    if index == 0 { "p/C" } else { "p/D" }
                );
            }
            assert_eq!(query.revision().as_i64(), 1);
            drop(query);
            assert_eq!(
                reader
                    .query_row("SELECT COUNT(DISTINCT revision) FROM entries", [], |row| {
                        row.get::<_, i64>(0)
                    })
                    .unwrap(),
                1
            );
        }
    }

    #[test]
    fn resource_failures_do_not_publish_partial_containers_or_stop_the_loop() {
        let root = tempdir().unwrap();
        let path = root.path().join("index.sqlite");
        let mut engine = Engine::new(&path).unwrap();
        engine.start().unwrap();
        let sender = engine.sender();
        for (name, bytes) in [
            (
                "broken.jar",
                jar(&[
                    ("p/C.class", class_bytes("p/C")),
                    ("broken.class", b"\xCA\xFE\xBA\xBE".to_vec()),
                ]),
            ),
            ("binary.java", b"\xCA\xFE\xBA\xBE".to_vec()),
            ("unknown", b"unsupported".to_vec()),
            ("empty.jar", jar(&[])),
        ] {
            let resource = root.path().join(name);
            std::fs::write(&resource, bytes).unwrap();
            send(&sender, ResourceId::file(resource));
        }
        send(&sender, ResourceId::file(root.path().join("missing")));
        send(&sender, ResourceId::file(root.path()));
        send(
            &sender,
            ResourceId::new(ResourceRoot::Virtual {
                provider: "test".into(),
                key: "input".into(),
            }),
        );
        send(
            &sender,
            ResourceId::file(root.path().join("broken.jar"))
                .entry(ResourceEntry("p/C.class".into())),
        );
        let source = root.path().join("Sentinel.java");
        std::fs::write(&source, "class Sentinel {}").unwrap();
        send(&sender, ResourceId::file(&source));
        let mut reader = reader(&path);
        wait_for_rows(&reader, 1);
        let query = ModelQuery::new(&mut reader).unwrap();
        assert!(
            query
                .java_file(&ResourceId::file(source))
                .unwrap()
                .is_some()
        );
        assert!(query.jvm_classes().unwrap().is_empty());
        assert_eq!(query.revision().as_i64(), 1);
    }

    #[test]
    fn document_and_event_writers_allocate_distinct_revisions() {
        let root = tempdir().unwrap();
        let path = root.path().join("index.sqlite");
        let mut engine = Engine::new(&path).unwrap();
        engine.start().unwrap();
        let mut resources = Vec::new();
        for index in 0..20 {
            let path = root.path().join(format!("binary-{index}"));
            std::fs::write(&path, class_bytes("p/C")).unwrap();
            resources.push(ResourceId::file(path));
        }
        let sender = engine.sender();
        let producer = thread::spawn(move || {
            for resource in resources {
                send(&sender, resource);
            }
        });
        let source = ResourceId::file(root.path().join("Document.java"));
        for index in 0..20 {
            engine
                .document_edit(
                    source.clone(),
                    "java",
                    &format!("class Document{index} {{}}"),
                )
                .unwrap()
                .unwrap();
        }
        producer.join().unwrap();
        let mut reader = reader(&path);
        wait_for_rows(&reader, 40);
        assert_eq!(
            reader
                .query_row("SELECT COUNT(DISTINCT revision) FROM entries", [], |row| {
                    row.get::<_, i64>(0)
                })
                .unwrap(),
            40
        );
        let query = ModelQuery::new(&mut reader).unwrap();
        assert_eq!(query.revision().as_i64(), 40);
        assert_eq!(query.jvm_classes().unwrap().len(), 20);
        assert_eq!(
            query.content(&source).unwrap().unwrap(),
            b"class Document19 {}"
        );
    }

    #[test]
    fn engine_shutdown_joins_the_worker_even_with_live_producers() {
        let root = tempdir().unwrap();
        let mut engine = Engine::new(&root.path().join("index.sqlite")).unwrap();
        engine.start().unwrap();
        let sender = engine.sender();
        let (done, completed) = crossbeam_channel::bounded(1);
        let dropping = thread::spawn(move || {
            drop(engine);
            done.send(()).unwrap();
        });
        completed
            .recv_timeout(Duration::from_secs(5))
            .expect("Engine shutdown waited for a producer to disconnect");
        dropping.join().unwrap();
        assert!(
            sender
                .send(Event::JavaHomeFound(JavaHomeFound {
                    home: root.path().into(),
                    version: None
                }))
                .is_err()
        );
    }

    #[test]
    fn workspace_start_feeds_the_engine_thread() {
        const ROOT: &str = "BEANS_ENGINE_EVENT_TEST_ROOT";
        if let Some(root) = std::env::var_os(ROOT) {
            let root = std::path::PathBuf::from(root);
            let path = root.join("index.sqlite");
            let mut engine = Engine::new(&path).unwrap();
            engine.start().unwrap();
            beans_workspace_toml::TomlWorkspace::start(&root, &engine.sender())
                .unwrap()
                .unwrap();
            let mut reader = reader(&path);
            wait_for_rows(&reader, 1);
            let query = ModelQuery::new(&mut reader).unwrap();
            let id = ResourceId::file(root.join("vendor/jdk/lib/modules"))
                .entry(ResourceEntry("/java.base/p/C.class".into()));
            assert_eq!(query.jvm_class(&id).unwrap().unwrap().name, "p/C");
            return;
        }
        let root = tempdir().unwrap();
        std::fs::create_dir_all(root.path().join("vendor/jdk/lib")).unwrap();
        std::fs::write(
            root.path().join("vendor/jdk/lib/modules"),
            jimage(&class_bytes("p/C")),
        )
        .unwrap();
        std::fs::write(
            root.path().join("beans.toml"),
            "jdk_home = 'vendor/jdk'\n[module.app]\nsources = ['src']\n",
        )
        .unwrap();
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "event::tests::workspace_start_feeds_the_engine_thread",
                "--nocapture",
            ])
            .env(ROOT, root.path())
            .env_remove("BEANS_WORKSPACE_TOML_JAVA_HOME")
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }

    fn processed(events: &Receiver<Event>, resource: &ResourceId) -> ResourceProcessed {
        let deadline = Instant::now() + Duration::from_secs(60);
        loop {
            let event = events
                .recv_deadline(deadline)
                .expect("timed out waiting for ResourceProcessed");
            if let Event::ResourceProcessed(completed) = event {
                assert_eq!(&completed.resource, resource);
                return completed;
            }
        }
    }

    fn send(sender: &crossbeam_channel::Sender<Event>, resource: ResourceId) {
        sender
            .send(Event::ResourceFound(ResourceFound { resource }))
            .unwrap();
    }

    fn reader(path: &Path) -> Connection {
        Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap()
    }

    fn wait_for_rows(reader: &Connection, expected: i64) {
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            let count: i64 = reader
                .query_row("SELECT COUNT(*) FROM entries", [], |row| row.get(0))
                .unwrap();
            if count >= expected {
                return;
            }
            assert!(
                Instant::now() < deadline,
                "expected {expected} rows, found {count}"
            );
            thread::sleep(Duration::from_millis(5));
        }
    }

    fn jar(entries: &[(&str, Vec<u8>)]) -> Vec<u8> {
        let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
        for (name, bytes) in entries {
            writer
                .start_file(*name, SimpleFileOptions::default())
                .unwrap();
            writer.write_all(bytes).unwrap();
        }
        writer.finish().unwrap().into_inner()
    }

    fn jimage(class: &[u8]) -> Vec<u8> {
        let strings = b"\0java.base\0p\0C\0class\0";
        let mut locations = vec![0, 0x08, 1, 0x10, 11, 0x18, 13, 0x20, 15, 0x3F];
        locations.extend((class.len() as u64).to_be_bytes());
        locations.push(0);
        let mut bytes = Vec::new();
        for word in [
            0xCAFEDADA,
            0x00010000,
            0,
            1,
            1,
            locations.len() as u32,
            strings.len() as u32,
            u32::MAX,
            1,
        ] {
            bytes.extend(word.to_le_bytes());
        }
        bytes.extend(locations);
        bytes.extend(strings);
        bytes.extend(class);
        bytes
    }
}
