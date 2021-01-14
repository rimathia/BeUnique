use futures::{FutureExt, StreamExt};
use std::sync::atomic::AtomicUsize;
use std::sync::Arc;
use std::{collections::HashMap, sync::atomic::Ordering};
use tokio::sync::{mpsc, RwLock};
use warp::{
    ws::{Message, WebSocket},
    Filter,
};

extern crate common;

/// Our global unique user id counter.
static NEXT_USER_ID: AtomicUsize = AtomicUsize::new(1);

#[tokio::main]
async fn main() {
    pretty_env_logger::init();

    let users = models::Users::default();

    let state = models::State::default();

    let routes = warp::path("test")
        // The `ws()` filter will prepare the Websocket handshake.
        .and(warp::ws())
        .and(filters::with_users(users))
        .and(filters::with_state(state))
        .map(|ws: warp::ws::Ws, users, state| {
            // And then our closure will be called when it completes...
            ws.on_upgrade(move |websocket| {
                user_connected(websocket, users, state)
                //// Just echo all messages back...
                //let (tx, rx) = websocket.split();
                //rx.forward(tx).map(|result| {
                //    if let Err(e) = result {
                //        eprintln!("websocket error: {:?}", e);
                //    }
                //})
            })
        });

    warp::serve(routes).run(([127, 0, 0, 1], 9001)).await;
}

async fn user_connected(websocket: WebSocket, users: models::Users, state: models::State) {
    // Use a counter to assign a new unique ID for this user.
    let my_id = NEXT_USER_ID.fetch_add(1, Ordering::Relaxed);

    eprintln!("new connected user: {}", my_id);

    let (user_ws_tx, mut user_ws_rx) = websocket.split();

    // Use an unbounded channel to handle buffering and flushing of messages
    // to the websocket...
    let (tx, rx) = mpsc::unbounded_channel();
    tokio::task::spawn(rx.forward(user_ws_tx).map(|result| {
        if let Err(e) = result {
            eprintln!("websocket send error: {}", e);
        }
    }));

    // Save the sender in our list of connected users.
    users.write().await.insert(my_id, tx);

    notify_all(&users, &state).await;

    // Return a `Future` that is basically a state machine managing
    // this specific user's connection.

    // Make an extra clone to give to our disconnection handler...
    let users2 = users.clone();

    // Every time the user sends a message, process it
    while let Some(result) = user_ws_rx.next().await {
        let msg = match result {
            Ok(msg) => msg,
            Err(e) => {
                eprintln!("websocket error(uid={}): {}", my_id, e);
                break;
            }
        };
        user_message(my_id, msg, &users, &state).await;
    }

    // user_ws_rx stream will keep processing as long as the user stays
    // connected. Once they disconnect, then...
    user_disconnected(my_id, &users2).await;
}

async fn notify_all(users: &models::Users, state: &models::State) {
    for (id, tx) in users.write().await.iter() {
        let view = state.lock().await.get_view(*id);
        let view_json = serde_json::to_string(&view).unwrap();
        if let Err(_disconnected) = tx.send(Ok(Message::text(view_json))) {
            // The tx is disconnected, our `user_disconnected` code
            // should be happening in another task, nothing more to
            // do here.
        }
    }
}

async fn user_message(my_id: usize, msg: Message, users: &models::Users, state: &models::State) {
    eprintln!("user {} has sent message {:?}", my_id, msg);

    match msg.to_str() {
        Ok(msg_text) => match serde_json::from_str::<common::game::Action>(msg_text) {
            Ok(action) => {
                state.lock().await.action(&action);
            }
            Err(e) => {
                eprintln!("error in deserializing: {:?}", e)
            }
        },
        Err(e) => {
            eprintln!("error in deserializing: {:?}", e)
        }
    }
    if msg.is_close() {
        eprintln!("marking id {} as disconnected", my_id);
        let disconnect = common::game::Action::DisconnectPlayer(my_id);
        state.lock().await.action(&disconnect);
    }

    notify_all(users, state).await
}

async fn user_disconnected(my_id: usize, users: &models::Users) {
    eprintln!("good bye user: {}", my_id);

    // Stream closed up, so remove from the user list
    users.write().await.remove(&my_id);
}

mod handlers {
    use super::models::State;
    use std::convert::Infallible;
}

mod models {
    use common::game;
    use std::collections::HashMap;
    use std::sync::Arc;
    use tokio::sync::{mpsc, Mutex, RwLock};
    use warp::ws::Message;

    pub type State = Arc<Mutex<game::State>>;

    /// Our state of currently connected users.
    ///
    /// - Key is their id
    /// - Value is a sender of `warp::ws::Message`
    pub type Users =
        Arc<RwLock<HashMap<usize, mpsc::UnboundedSender<Result<Message, warp::Error>>>>>;
}

mod filters {
    use super::models::{State, Users};
    use warp::Filter;

    pub fn with_state(
        state: State,
    ) -> impl Filter<Extract = (State,), Error = std::convert::Infallible> + Clone {
        warp::any().map(move || state.clone())
    }

    pub fn with_users(
        users: Users,
    ) -> impl Filter<Extract = (Users,), Error = std::convert::Infallible> + Clone {
        warp::any().map(move || users.clone())
    }
}