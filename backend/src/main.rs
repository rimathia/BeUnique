use rand::thread_rng;
use std::collections;
use std::fs;
use std::io::BufRead;
use std::io::BufReader;
use std::string::String;
use std::sync::atomic::AtomicUsize;
use warp::Filter;

extern crate common;

/// Our global unique user id counter.
static NEXT_USER_ID: AtomicUsize = AtomicUsize::new(1);

#[tokio::main]
async fn main() {
    pretty_env_logger::init();

    let users = models::Users::default();
    let filename = "words.txt";
    let maybe_words: Result<std::vec::Vec<std::string::String>, std::io::Error> =
        std::fs::File::open(filename)
            .and_then(|f| Ok(BufReader::new(f).lines().filter_map(Result::ok).collect()));

    use common::game::Dictionary;
    use rand::seq::SliceRandom;
    let dictionary = match maybe_words {
        Ok(mut words) => {
            if words.len() > 0 {
                let mut rng = thread_rng();
                words.shuffle(&mut rng);
                Dictionary::new(words)
            } else {
                eprintln!("read 0 words from {:?}, using default dictionary", filename);
                Dictionary::default()
            }
        }
        Err(e) => {
            eprintln!(
                "couldn't read words from {:?} because of {:?}, using default dictionary",
                filename, e
            );
            Dictionary::default()
        }
    };

    let state = models::State::new(tokio::sync::Mutex::new(common::game::State::new(
        dictionary,
    )));

    let routes = warp::path("test")
        // The `ws()` filter will prepare the Websocket handshake.
        .and(warp::ws())
        .and(filters::with_users(users))
        .and(filters::with_state(state.clone()))
        .map(|ws: warp::ws::Ws, users, state| {
            // And then our closure will be called when it completes...
            ws.on_upgrade(move |websocket| {
                handlers::user_connected(websocket, users, state)
                //// Just echo all messages back...
                //let (tx, rx) = websocket.split();
                //rx.forward(tx).map(|result| {
                //    if let Err(e) = result {
                //        eprintln!("websocket error: {:?}", e);
                //    }
                //})
            })
        })
        .or(warp::path("debug")
            .and(filters::with_state(state.clone()))
            .map(|state: models::State| format!("{:#?}", state)));

    warp::serve(routes).run(([127, 0, 0, 1], 9001)).await;
}

mod handlers {
    use super::models::{State, Users};
    use futures::{FutureExt, StreamExt};
    use std::sync::atomic::Ordering;
    use tokio::sync::mpsc;
    use warp::ws::Message;
    use warp::ws::WebSocket;

    pub async fn user_connected(websocket: WebSocket, users: Users, state: State) {
        // Use a counter to assign a new unique ID for this user.
        let my_id = super::NEXT_USER_ID.fetch_add(1, Ordering::Relaxed);

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
        user_disconnected(my_id, &users2, &state).await;
    }

    async fn notify_all(users: &Users, state: &State) {
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

    pub async fn user_message(my_id: usize, msg: Message, users: &Users, state: &State) {
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

    async fn user_disconnected(my_id: usize, users: &Users, state: &State) {
        eprintln!("good bye user: {}", my_id);

        // Stream closed up, so remove from the user list
        eprintln!("marking id {} as disconnected", my_id);
        let disconnect = common::game::Action::DisconnectPlayer(my_id);
        state.lock().await.action(&disconnect);
        users.write().await.remove(&my_id);
    }
}

mod server {
    // mod game {
    //     use common::game::{GamePhase, Player};

    //     #[derive(Serialize, Deserialize, Debug, Clone)]
    //     pub struct Dictionary {
    //         candidate_words: Vec<String>,
    //         used_words: Vec<String>,
    //     }

    //     impl Default for Dictionary {
    //         fn default() -> Self {
    //             Dictionary {
    //                 candidate_words: vec!["Steinbruch".to_string(), "Schwan".to_string()],
    //                 used_words: vec![],
    //             }
    //         }
    //     }

    //     impl Dictionary {
    //         pub fn new(mut words: Vec<String>) -> Self {
    //             use rand::seq::SliceRandom;
    //             let mut rng = rand::thread_rng();
    //             words.shuffle(&mut rng);
    //             Self {
    //                 candidate_words: words,
    //                 used_words: vec![],
    //             }
    //         }
    //         fn get_word(&mut self) -> String {
    //             if self.candidate_words.is_empty() {
    //                 std::mem::swap(&mut self.candidate_words, &mut self.used_words);
    //                 use rand::seq::SliceRandom;
    //                 let mut rng = rand::thread_rng();
    //                 self.candidate_words.shuffle(&mut rng);
    //             }
    //             let word = self.candidate_words.pop().unwrap();
    //             self.used_words.push(word.clone());
    //             word
    //         }
    //     }

    //     #[derive(Serialize, Deserialize, Debug, Clone)]
    //     pub struct State {
    //         pub players: Vec<Player>,
    //         pub active_index: Option<usize>,
    //         pub phase: GamePhase,
    //         pub past_rounds: Vec<(Player, String, bool)>,
    //         pub dictionary: Dictionary,
    //     }
    //     impl State {
    //         pub fn new(dictionary: Dictionary) -> Self {
    //             Self {
    //                 players: vec![],
    //                 active_index: None,
    //                 phase: GamePhase::default(),
    //                 past_rounds: vec![],
    //                 dictionary: dictionary,
    //             }
    //         }

    //         pub fn action(&mut self, action: &Action) -> Option<()> {
    //             match action {
    //                 Action::Join(new_id, new_name) => self.join(*new_id, new_name),
    //                 Action::DisconnectPlayer(id) => self.disconnect_player(*id),
    //                 Action::Start(_id) => self.start(),
    //                 Action::GiveHint(id, hint) => self.process_hint(*id, hint),
    //                 Action::FilterHint(id, hint, valid) => {
    //                     self.process_hint_filter(*id, hint, *valid)
    //                 }
    //                 Action::FinishHintFiltering(id) => self.process_finish_hint_filter(*id),
    //                 Action::Guess(id, guess) => self.process_guess(*id, guess),
    //                 Action::Judge(id, correct) => self.process_guess_judgement(*id, *correct),
    //                 Action::FinishJudging(id) => self.process_finish_judging(*id),
    //             }
    //         }

    //         pub fn disconnect_player(&mut self, disconnect_id: usize) -> Option<()> {
    //             self.players
    //                 .iter_mut()
    //                 .find(|p| p.id == Some(disconnect_id))?
    //                 .id = None;
    //             Some(())
    //         }

    //         pub fn join(&mut self, new_id: usize, new_name: &str) -> Option<()> {
    //             match self.players.iter_mut().find(|p| p.name == new_name) {
    //                 Some(player) => {
    //                     if player.id.is_none() {
    //                         player.id = Some(new_id);
    //                     }
    //                     Some(())
    //                 }
    //                 None => {
    //                     self.players.push(Player {
    //                         id: Some(new_id),
    //                         name: new_name.to_string(),
    //                     });
    //                     if self.active_index.is_none() {
    //                         self.active_index = Some(self.players.len() - 1);
    //                     }
    //                     Some(())
    //                 }
    //             }
    //         }

    //         fn player_index(&self, id: usize) -> Option<usize> {
    //             self.players.iter().position(|p| p.id == Some(id))
    //         }

    //         fn player(&self, id: usize) -> Option<&Player> {
    //             self.players.iter().find(|p| p.id == Some(id))
    //         }

    //         fn start(&mut self) -> Option<()> {
    //             match &mut self.phase {
    //                 GamePhase::GatherPlayers => {
    //                     self.phase = GamePhase::HintCollection(HintCollection {
    //                         word: self.dictionary.get_word(),
    //                         hints: HashMap::new(),
    //                     });
    //                     Some(())
    //                 }
    //                 _ => {
    //                     eprintln!("start in wrong game state");
    //                     None
    //                 }
    //             }
    //         }

    //         fn process_hint(&mut self, id: usize, hint: &str) -> Option<()> {
    //             let submitter_index = self.player_index(id)?;
    //             let submitter = &self.players[submitter_index];
    //             match &mut self.phase {
    //                 GamePhase::HintCollection(hint_collection) => {
    //                     if submitter_index != self.active_index? {
    //                         hint_collection.hints.insert(
    //                             submitter.name.clone(),
    //                             Hint {
    //                                 content: hint.to_string(),
    //                                 allowed: true,
    //                             },
    //                         );
    //                         if hint_collection.hints.len() == self.players.len() - 1 {
    //                             self.phase = GamePhase::HintFiltering(HintFiltering {
    //                                 word: hint_collection.word.clone(),
    //                                 hints: hint_collection.hints.clone(),
    //                             });
    //                         }
    //                         Some(())
    //                     } else {
    //                         eprintln!("hint from active player? {:?}", (id, hint));
    //                         None
    //                     }
    //                 }
    //                 _ => {
    //                     eprintln!("hint in wrong game state");
    //                     None
    //                 }
    //             }
    //         }

    //         fn process_hint_filter(&mut self, id: usize, hint: &str, allowed: bool) -> Option<()> {
    //             let active = self.player_index(id)? == self.active_index?;
    //             match &mut self.phase {
    //                 GamePhase::HintFiltering(HintFiltering { word: _, hints }) => {
    //                     if !active {
    //                         hints.get_mut(hint)?.allowed = allowed;
    //                         Some(())
    //                     } else {
    //                         eprintln!("hint filtering from active player");
    //                         None
    //                     }
    //                 }
    //                 _ => {
    //                     eprintln!("hint filtering in wrong game state");
    //                     None
    //                 }
    //             }
    //         }

    //         fn process_finish_hint_filter(&mut self, id: usize) -> Option<()> {
    //             let active = self.player_index(id)? == self.active_index?;
    //             match &mut self.phase {
    //                 GamePhase::HintFiltering(HintFiltering { word, hints }) => {
    //                     if !active {
    //                         self.phase = GamePhase::Guessing(Guessing {
    //                             word: word.clone(),
    //                             hints: hints.clone(),
    //                             guess: None,
    //                         });
    //                         Some(())
    //                     } else {
    //                         eprintln!("finish hint filtering from active player");
    //                         None
    //                     }
    //                 }
    //                 _ => {
    //                     eprintln!("finish hint filtering in wrong game state");
    //                     None
    //                 }
    //             }
    //         }

    //         fn process_guess(&mut self, id: usize, input_guess: &str) -> Option<()> {
    //             let active = self.player_index(id)? == self.active_index?;
    //             match &mut self.phase {
    //                 GamePhase::Guessing(Guessing { word, hints, guess }) => {
    //                     if active {
    //                         *guess = Some(input_guess.to_string());
    //                         self.phase = GamePhase::Judging(Judging {
    //                             word: word.clone(),
    //                             hints: hints.clone(),
    //                             guess: guess.clone(),
    //                             success: None,
    //                         });
    //                         Some(())
    //                     } else {
    //                         eprintln!("guess from inactive player");
    //                         None
    //                     }
    //                 }
    //                 _ => {
    //                     eprintln!("guess in wrong game state");
    //                     None
    //                 }
    //             }
    //         }

    //         fn process_guess_judgement(&mut self, id: usize, correct: bool) -> Option<()> {
    //             let active = self.player_index(id)? == self.active_index?;
    //             match &mut self.phase {
    //                 GamePhase::Judging(Judging {
    //                     word: _,
    //                     hints: _,
    //                     guess: _,
    //                     success,
    //                 }) => {
    //                     if !active {
    //                         *success = Some(correct);
    //                         Some(())
    //                     } else {
    //                         eprintln!("judgement from active player");
    //                         None
    //                     }
    //                 }
    //                 _ => {
    //                     eprintln!("guess judgement in wrong game state");
    //                     None
    //                 }
    //             }
    //         }

    //         fn process_finish_judging(&mut self, _id: usize) -> Option<()> {
    //             match &mut self.phase {
    //                 GamePhase::Judging(Judging {
    //                     word,
    //                     hints: _,
    //                     guess: _,
    //                     success,
    //                 }) => {
    //                     let active_player = &self.players[self.active_index?];
    //                     self.past_rounds.push((
    //                         active_player.clone(),
    //                         word.clone(),
    //                         success.unwrap_or(false),
    //                     ));
    //                     self.active_index = Some((self.active_index? + 1) % self.players.len());
    //                     self.phase = GamePhase::HintCollection(HintCollection {
    //                         word: self.dictionary.get_word(),
    //                         hints: HashMap::new(),
    //                     });
    //                     Some(())
    //                 }
    //                 _ => {
    //                     eprintln!("finish judging in wrong game state");
    //                     None
    //                 }
    //             }
    //         }

    //         pub fn list_actions(&self, id: usize) -> Vec<Action> {
    //             match self.player_index(id) {
    //                 Some(i) => {
    //                     let active = i == self.active_index.unwrap_or(0);
    //                     match &self.phase {
    //                         GamePhase::GatherPlayers => {
    //                             if self.players.len() >= 2 {
    //                                 vec![Action::Start(id)]
    //                             } else {
    //                                 vec![]
    //                             }
    //                         }
    //                         GamePhase::HintCollection(_) => {
    //                             if !active {
    //                                 vec![Action::GiveHint(id, String::new())]
    //                             } else {
    //                                 vec![]
    //                             }
    //                         }
    //                         GamePhase::HintFiltering(HintFiltering { word: _, hints }) => {
    //                             if !active {
    //                                 let mut actions: Vec<Action> = hints
    //                                     .iter()
    //                                     .map(|(_, hint)| {
    //                                         Action::FilterHint(
    //                                             id,
    //                                             hint.content.clone(),
    //                                             hint.allowed,
    //                                         )
    //                                     })
    //                                     .collect();
    //                                 actions.push(Action::FinishHintFiltering(id));
    //                                 actions
    //                             } else {
    //                                 vec![]
    //                             }
    //                         }
    //                         GamePhase::Guessing(_) => {
    //                             if !active {
    //                                 vec![]
    //                             } else {
    //                                 vec![Action::Guess(id, String::new())]
    //                             }
    //                         }
    //                         GamePhase::Judging(_) => {
    //                             if !active {
    //                                 vec![Action::Judge(id, true), Action::FinishJudging(id)]
    //                             } else {
    //                                 vec![]
    //                             }
    //                         }
    //                     }
    //                 }
    //                 None => {
    //                     vec![Action::Join(id, String::new())]
    //                 }
    //             }
    //         }
    //     }
    // }
}
mod models {
    use std::collections::HashMap;
    use std::sync::Arc;
    use tokio::sync::{mpsc, Mutex, RwLock};
    use warp::ws::Message;

    pub type State = Arc<Mutex<common::game::State>>;

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
