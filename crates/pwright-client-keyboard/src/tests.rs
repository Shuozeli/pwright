use super::*;
use pwright_client::{
    AtomicCallIdSource, BrowserContextKind, BrowserKind, Client, PageKind, RemoteInitializer,
    RemoteKind,
};
use pwright_core::{CallId, CancellationId, CommandSpec, MethodTag, ObjectId};
use std::{
    future::{Ready, ready},
    sync::{Arc, Mutex},
    time::Duration,
};

#[derive(Clone, Debug, Eq, PartialEq)]
struct Record {
    method: MethodTag,
    page: ObjectId,
    id: CallId,
    metadata: CallMetadata,
}
struct Channel(Arc<Mutex<Vec<Record>>>);
struct DownOnly(Arc<Mutex<Vec<Record>>>);

macro_rules! handler {
    ($channel:ty, $command:ty) => {
        impl Service<Command<$command>> for $channel {
            type Response = CommandResult<$command>;
            type Error = Infallible;
            type Future<'a> = Ready<Result<Self::Response, Self::Error>>;
            fn call(&self, command: Command<$command>) -> Self::Future<'_> {
                self.0.lock().unwrap().push(Record {
                    method: <$command>::METHOD,
                    page: command.object_id(),
                    id: command.call_id(),
                    metadata: command.metadata(),
                });
                ready(Ok(CommandResult::new(command.call_id(), Ok(()))))
            }
        }
    };
}
handler!(Channel, KeyboardDown);
handler!(Channel, KeyboardUp);
handler!(Channel, KeyboardPress);
handler!(Channel, KeyboardInsertText);
handler!(Channel, KeyboardType);
handler!(DownOnly, KeyboardDown);

type TestClient<C> = Client<C, AtomicCallIdSource>;
fn page<C>(channel: C) -> (TestClient<C>, Page<C, AtomicCallIdSource>) {
    let client = Client::new(channel, AtomicCallIdSource::new(10));
    let browser = client
        .browser(RemoteInitializer::new(
            ObjectId::new(1),
            None,
            BrowserKind::OBJECT_KIND,
        ))
        .unwrap();
    let context = browser
        .context(RemoteInitializer::new(
            ObjectId::new(2),
            Some(ObjectId::new(1)),
            BrowserContextKind::OBJECT_KIND,
        ))
        .unwrap();
    let page = context
        .page(RemoteInitializer::new(
            ObjectId::new(3),
            Some(ObjectId::new(2)),
            PageKind::OBJECT_KIND,
        ))
        .unwrap();
    (client, page)
}

#[tokio::test]
async fn each_keyboard_extension_dispatches_to_page_without_a_main_frame() {
    // Arrange
    let recorded = Arc::new(Mutex::new(Vec::new()));
    let (_client, page) = page(Channel(recorded.clone()));
    let metadata = CallMetadata::new(Some(Duration::new(3, 42)), Some(CancellationId::new(13)));
    let key = KeyboardKey::copy_from_str("Shift+").unwrap();
    let text = KeyboardText::copy_from_str("hello").unwrap();
    // Act
    page.keyboard_down(DownParams { key: key.clone() }, metadata)
        .await
        .unwrap();
    page.keyboard_up(UpParams { key: key.clone() }, metadata)
        .await
        .unwrap();
    page.keyboard_press(PressParams { key, delay: None }, metadata)
        .await
        .unwrap();
    page.keyboard_insert_text(InsertTextParams { text: text.clone() }, metadata)
        .await
        .unwrap();
    page.keyboard_type(TypeParams { text, delay: None }, metadata)
        .await
        .unwrap();
    // Assert
    assert!(page.main_frame().is_none());
    let expected: Vec<_> = [
        KeyboardDown::METHOD,
        KeyboardUp::METHOD,
        KeyboardPress::METHOD,
        KeyboardInsertText::METHOD,
        KeyboardType::METHOD,
    ]
    .into_iter()
    .zip(10..)
    .map(|(method, id)| Record {
        method,
        page: ObjectId::new(3),
        id: CallId::new(id),
        metadata,
    })
    .collect();
    assert_eq!(*recorded.lock().unwrap(), expected);
}

#[tokio::test]
async fn down_extension_does_not_require_any_other_keyboard_command() {
    // Arrange
    let recorded = Arc::new(Mutex::new(Vec::new()));
    let (_client, page) = page(DownOnly(recorded.clone()));
    // Act
    page.keyboard_down(
        DownParams {
            key: KeyboardKey::copy_from_str("").unwrap(),
        },
        CallMetadata::default(),
    )
    .await
    .unwrap();
    // Assert
    assert_eq!(recorded.lock().unwrap().len(), 1);
    assert_eq!(recorded.lock().unwrap()[0].method, KeyboardDown::METHOD);
}
