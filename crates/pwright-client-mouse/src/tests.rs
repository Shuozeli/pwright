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
handler!(Channel, MouseMove);
handler!(Channel, MouseDown);
handler!(Channel, MouseUp);
handler!(Channel, MouseClick);
handler!(Channel, MouseDblclick);
handler!(Channel, MouseWheel);
handler!(DownOnly, MouseDown);

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

fn number(value: f64) -> MouseNumber {
    MouseNumber::try_from(value).unwrap()
}

#[tokio::test]
async fn each_mouse_extension_dispatches_to_page_without_main_frame() {
    // Arrange
    let recorded = Arc::new(Mutex::new(Vec::new()));
    let (_client, page) = page(Channel(recorded.clone()));
    let metadata = CallMetadata::new(Some(Duration::new(3, 42)), Some(CancellationId::new(13)));
    // Act
    page.mouse_move(
        MoveParams {
            x: number(12.5),
            y: number(-24.25),
            steps: MouseMoveSteps::try_from(-2.0).unwrap(),
        },
        metadata,
    )
    .await
    .unwrap();
    page.mouse_down(
        DownParams {
            button: MouseButton::Right,
            click_count: -2,
        },
        metadata,
    )
    .await
    .unwrap();
    page.mouse_up(
        UpParams {
            button: MouseButton::Middle,
            click_count: 2,
        },
        metadata,
    )
    .await
    .unwrap();
    page.mouse_click(
        ClickParams {
            x: number(5.5),
            y: number(8.25),
            button: MouseButton::Left,
            click_count: MouseClickCount::try_from(3.0).unwrap(),
            delay: Some(std::time::Duration::new(1, 500_000)),
        },
        metadata,
    )
    .await
    .unwrap();
    page.mouse_dblclick(
        DblclickParams {
            x: number(5.5),
            y: number(8.25),
            button: MouseButton::Right,
            delay: None,
        },
        metadata,
    )
    .await
    .unwrap();
    page.mouse_wheel(
        WheelParams {
            delta_x: number(-1.5),
            delta_y: number(90.25),
        },
        metadata,
    )
    .await
    .unwrap();
    // Assert
    assert!(page.main_frame().is_none());
    let expected: Vec<_> = [
        MouseMove::METHOD,
        MouseDown::METHOD,
        MouseUp::METHOD,
        MouseClick::METHOD,
        MouseDblclick::METHOD,
        MouseWheel::METHOD,
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
async fn down_only_channel_does_not_require_other_mouse_services() {
    // Arrange
    let recorded = Arc::new(Mutex::new(Vec::new()));
    let (_client, page) = page(DownOnly(recorded.clone()));
    // Act
    page.mouse_down(
        DownParams {
            button: MouseButton::Right,
            click_count: -2,
        },
        CallMetadata::default(),
    )
    .await
    .unwrap();
    // Assert
    assert_eq!(recorded.lock().unwrap().len(), 1);
    assert_eq!(recorded.lock().unwrap()[0].method, MouseDown::METHOD);
}
