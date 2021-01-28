#![recursion_limit = "256"]

use anyhow::Error;
use wasm_bindgen::prelude::*;
use yew::format::Json;
use yew::prelude::*;
use yew::services::websocket::{WebSocketService, WebSocketStatus, WebSocketTask};

extern crate common;
use common::game;

struct Model {
    link: ComponentLink<Self>,
    state: game::PlayerView,
    ws: Option<WebSocketTask>,
}

pub enum WsAction {
    Connect,
    Disconnect,
    Lost,
}

//#[derive(Serialize, Deserialize, Debug)]
//pub struct WsResponse {
//    value: i64,
//}

enum Msg {
    // Add(i64),
    Ignore,
    WsAction(WsAction),
    WsReady(Result<game::PlayerView, Error>),
    WsSend(common::game::Action),
}

impl From<WsAction> for Msg {
    fn from(action: WsAction) -> Self {
        Msg::WsAction(action)
    }
}

// struct GamePhase(game::VisibleGamePhase);

// impl Component for GamePhase {
//     fn view(&self) -> Html {
//         match &self.0 {
//             game::VisibleGamePhase::GatherPlayers => {
//                 html! { { "Das Spiel hat noch nicht angefangen." } }
//             }
//             game::VisibleGamePhase::HintCollection(game::VisibleHintCollection::Active(
//                 hint_collection,
//             )) => {
//                 html! {
//                     { format!(
//                         "Es sind schon {} Hinweise eingegangen.",
//                         hint_collection.players_done.len()
//                     ) }
//                 }
//             }
//             game::VisibleGamePhase::HintCollection(game::VisibleHintCollection::Inactive(
//                 hint_collection,
//             )) => html! { { format!(
//                 "Bitte gib einen Hinweis für das Wort {}.",
//                 hint_collection.word
//             ) } },
//             game::VisibleGamePhase::HintFiltering(game::VisibleHintFiltering::Active(
//                 hint_filtering,
//             )) => html! { { format!(
//                 "Es werden doppelte Hinweise entfernt, aktuell sind {} übrig.",
//                 hint_filtering.players_valid_hints.len()
//             ) } },
//             game::VisibleGamePhase::HintFiltering(game::VisibleHintFiltering::Inactive(_)) => {
//                 html! { { format!("Welche Hinweise sind gültig?") } }
//             }
//             game::VisibleGamePhase::Guessing(game::VisibleGuessing::Active(guessing)) => {
//                 let render_hint = |hint: &game::VisibleHint| {
//                     html! {
//                         <div>
//                              { hint.0.clone() }
//                         </div>
//                     }
//                 };
//                 html! {
//                     <>
//                     { "Die Hinweise sind:" }
//                     <ul class="item-list">
//                         { for guessing.hints.iter().map(|(_, h)|{ render_hint(h) } ) }
//                     </ul>
//                     { "Welches Wort ist gesucht?" }
//                     </>
//                 }
//             }
//             game::VisibleGamePhase::Guessing(game::VisibleGuessing::Inactive(_)) => {
//                 html! { { "Jetzt wird geraten." } }
//             }
//             _ => html! { { "Noch nicht implementiert." } },
//         }
//     }
//
//     type Message = ();
//
//     type Properties = ();
//
//     fn create(props: Self::Properties, link: ComponentLink<Self>) -> Self {
//         todo!()
//     }
//
//     fn update(&mut self, msg: Self::Message) -> ShouldRender {
//         todo!()
//     }
//
//     fn change(&mut self, _props: Self::Properties) -> ShouldRender {
//         todo!()
//     }
// }

impl Component for Model {
    type Message = Msg;
    type Properties = ();
    fn create(_: Self::Properties, link: ComponentLink<Self>) -> Self {
        Self {
            link,
            state: game::PlayerView::default(),
            ws: None,
        }
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        match msg {
            Msg::Ignore => false,
            Msg::WsAction(action) => {
                match action {
                    WsAction::Connect => {
                        let callback = self.link.callback(|Json(data)| Msg::WsReady(data));
                        let notification = self.link.callback(|status| match status {
                            WebSocketStatus::Opened => Msg::Ignore,
                            WebSocketStatus::Closed | WebSocketStatus::Error => {
                                WsAction::Lost.into()
                            }
                        });
                        let task = WebSocketService::connect(
                            "ws://localhost:9001/test",
                            callback,
                            notification,
                        )
                        .unwrap();
                        self.ws = Some(task);
                    }
                    WsAction::Disconnect => {
                        self.ws.take();
                    }
                    WsAction::Lost => {
                        self.ws = None;
                    }
                };
                true
            }
            Msg::WsReady(response) => {
                match response {
                    Ok(new_state) => self.state = new_state,
                    Err(_) => {}
                }
                true
            }
            Msg::WsSend(action) => {
                match &mut self.ws {
                    Some(task) => {
                        let serialized = serde_json::to_string(&action).ok().unwrap();
                        task.send(Ok(serialized));
                    }
                    None => {}
                }
                false
            }
        }
    }

    fn change(&mut self, _props: Self::Properties) -> ShouldRender {
        // Should only return "true" if new properties are different to
        // previously received properties.
        // This component has no properties so we will always return "false".
        false
    }

    fn view(&self) -> Html {
        let to_html = |action: &common::game::Action| match action {
            common::game::Action::Join(id, _) => {
                let cloned_id: usize = *id;
                let send_name = move |e: ChangeData| match e {
                    ChangeData::Value(value) => {
                        Msg::WsSend(common::game::Action::Join(cloned_id, value))
                    }
                    _ => Msg::Ignore,
                };
                html! {
                        <div>
                            <label for="uname">{ "Mein Name:" }</label>
                            <input type="text" id="uname" name="name" onchange=self.link.callback(send_name)/>
                        </div>
                }
            }
            common::game::Action::DisconnectPlayer(_id) => {
                html! {
                    <div>
                        { "Das sollte nicht passieren (Verbindung explizit getrennt). Bitte neu laden." }
                    </div>
                }
            }
            common::game::Action::Start(id) => {
                let cloned_id: usize = *id;
                let send_start = move |_| Msg::WsSend(common::game::Action::Start(cloned_id));
                html! {
                    <div>
                        <button onclick=self.link.callback(send_start)>
                        {"Start"}
                        </button>
                    </div>
                }
            }
            common::game::Action::GiveHint(id, _hint) => {
                let cloned_id: usize = *id;
                let send_hint = move |e: ChangeData| match e {
                    ChangeData::Value(value) => {
                        Msg::WsSend(common::game::Action::GiveHint(cloned_id, value))
                    }
                    _ => Msg::Ignore,
                };
                html! {
                    <div>
                        <label for="hint">{ "Hinweis:" }</label>
                        <input type="text" id="hint" name="hint" onchange=self.link.callback(send_hint)/>
                    </div>
                }
            }
            common::game::Action::FilterHint(id, hint, valid) => {
                let send_flip_hint_validity = {
                    let id = *id;
                    let hint = hint.clone();
                    let valid = *valid;
                    move |_| {
                        let hint = hint.clone();
                        Msg::WsSend(common::game::Action::FilterHint(id, hint, !valid))
                    }
                };
                let flip_label = {
                    if *valid {
                        "ungültig".to_string()
                    } else {
                        "gültig".to_string()
                    }
                };
                html! {
                    <div>
                        {hint}
                        <button onclick=self.link.callback(send_flip_hint_validity)>
                        {flip_label}
                        </button>
                    </div>
                }
            }
            common::game::Action::FinishHintFiltering(id) => {
                let id = *id;
                let send_finish_filtering =
                    move |_| Msg::WsSend(common::game::Action::FinishHintFiltering(id));
                html! {
                    <div>
                        <button onclick=self.link.callback(send_finish_filtering)>
                        {"Hinweisbeurteilung abschliessen."}
                        </button>
                    </div>
                }
            }
            common::game::Action::Guess(id, _) => {
                let id: usize = *id;
                let send_guess = move |e: ChangeData| match e {
                    ChangeData::Value(value) => {
                        Msg::WsSend(common::game::Action::Guess(id, Some(value)))
                    }
                    _ => Msg::Ignore,
                };
                let send_no_guess = move |_| Msg::WsSend(common::game::Action::Guess(id, None));
                html! {
                    <>
                    <div>
                        <label for="hint">{ "Ich rate:" }</label>
                        <input type="text" id="guess" name="guess" onchange=self.link.callback(send_guess)/>
                    </div>
                    <div>
                        <button onclick=self.link.callback(send_no_guess)>
                        {"Keine Ahnung"}
                        </button>
                    </div>
                    </>
                }
            }
            common::game::Action::Judge(id, correct) => {
                let send_flip_guess_validity = {
                    let id = *id;
                    let correct = *correct;
                    move |_| Msg::WsSend(common::game::Action::Judge(id, correct))
                };
                let flip_label = {
                    if *correct {
                        "Antwort für richtig erklären".to_string()
                    } else {
                        "Antwort für falsch erklären".to_string()
                    }
                };
                html! {
                    <div>
                        <button onclick=self.link.callback(send_flip_guess_validity)>
                        {flip_label}
                        </button>
                    </div>
                }
            }
            common::game::Action::FinishJudging(id) => {
                let id = *id;
                let send_finish_judging =
                    move |_| Msg::WsSend(common::game::Action::FinishJudging(id));
                html! {
                    <div>
                        <button onclick=self.link.callback(send_finish_judging)>
                        {"Runde abschliessen"}
                        </button>
                    </div>
                }
            }
        };

        let action_html = self
            .state
            .actions
            .iter()
            .map(|action| to_html(action))
            .collect::<Vec<Html>>();

        let state_html = {
            let is_me = |player: &game::Player| match &self.state.me {
                Some(me) => me.name == player.name,
                None => false,
            };
            let list_item = |p: &game::Player| {
                let mut content = p.name.clone();
                if is_me(p) {
                    content += " (ich)";
                }
                if p.id.is_none() {
                    content += " (Verbindung verloren)"
                }
                html! {
                    <li>
                    { content }
                    </li>
                }
            };
            let list_players = if self.state.players.len() > 0 {
                html! {
                    <p>
                        { "Es spielen mit:" }
                        <ul class="item-list">
                            { for self.state.players.iter().map(list_item) }
                        </ul>
                    </p>
                }
            } else {
                html! {
                    <p>
                        { "Es ist noch niemand hier." }
                    </p>
                }
            };
            if self.state.me.is_none() {
                html! {
                    <>
                    <p>
                        { "Teilnehmen?" }
                    </p>
                        { list_players }
                    </>
                }
            } else {
                html! {
                    <>
                        { list_players }
                    </>
                }
            }
        };

        let past_rounds_html = if self.state.past_rounds.len() == 0 {
            html! {}
        } else {
            let n_success = self.state.past_rounds.iter().filter(|p| p.success).count();
            let n_total = self.state.past_rounds.len();
            let summary = format!(
                "Es wurde in {} von {} Runden richtig geraten.",
                n_success, n_total
            );
            let list_item = |p: &game::PastRound| {
                let verdict = if p.success {
                    format!("{} hat \"{}\" erraten.", p.name, p.word)
                } else {
                    format!("{} hat \"{}\" nicht erraten.", p.name, p.word)
                };
                html! {
                    <li> { verdict } </li>
                }
            };
            let list_rounds = html! {
                <p>
                    <ul class="item-list">
                        { for self.state.past_rounds.iter().map(list_item) }
                    </ul>
                </p>
            };
            html! {
                <>
                    <p>
                    { "Was bisher geschah: "}
                    </p>
                    { summary }
                    { list_rounds }
                </>
            }
        };

        let prelude = match &self.state.phase {
            game::VisibleGamePhase::GatherPlayers => {
                html! { { "Das Spiel hat noch nicht angefangen." } }
            }
            game::VisibleGamePhase::HintCollection(game::VisibleHintCollection::Active(
                hint_collection,
            )) => {
                let message = if hint_collection.players_done.len() > 0 {
                    format!(
                        "Es sind schon {} Hinweise eingegangen.",
                        hint_collection.players_done.len()
                    )
                } else {
                    format!("Es sind noch keine Hinweise eingegangen.")
                };
                html! {
                    { message }
                }
            }
            game::VisibleGamePhase::HintCollection(game::VisibleHintCollection::Inactive(
                hint_collection,
            )) => html! { { format!(
                "Bitte gib einen Hinweis für \"{}\".",
                hint_collection.word
            ) } },
            game::VisibleGamePhase::HintFiltering(game::VisibleHintFiltering::Active(
                hint_filtering,
            )) => html! { { format!(
                "Es werden doppelte Hinweise entfernt, aktuell sind {} übrig.",
                hint_filtering.players_valid_hints.len()
            ) } },
            game::VisibleGamePhase::HintFiltering(game::VisibleHintFiltering::Inactive(
                hint_filtering,
            )) => {
                html! { { format!("Welche Hinweise für \"{}\" sind gültig?", hint_filtering.word) } }
            }
            game::VisibleGamePhase::Guessing(game::VisibleGuessing::Active(guessing)) => {
                let render_hint = |hint: &game::VisibleHint| {
                    html! {
                        <div>
                             { hint.0.clone() }
                        </div>
                    }
                };
                html! {
                    <>
                    { "Die Hinweise sind:" }
                    <ul class="item-list">
                        { for guessing.hints.iter().map(|(_, h)|{ render_hint(h) } ) }
                    </ul>
                    { "Welches Wort ist gesucht?" }
                    </>
                }
            }
            game::VisibleGamePhase::Guessing(game::VisibleGuessing::Inactive(_)) => {
                html! { { "Wir warten bis geraten wurde." } }
            }
            game::VisibleGamePhase::Judging(game::VisibleJudging::Active(judging))
            | game::VisibleGamePhase::Judging(game::VisibleJudging::Inactive(judging)) => {
                let word = html! {
                    <p>
                        {format!("Das gesuchte Wort war {}.", judging.word)}
                    </p>
                };
                let guess = match &judging.guess {
                    Some(guess) => {
                        let literal_guess = if judging.success.unwrap_or(false) {
                            html! { {guess} }
                        } else {
                            html! { <s> {guess} </s> }
                        };
                        html! {
                            <>
                            <p>
                                {{"Geraten:"}}
                            </p>
                            <p>
                                { literal_guess }
                            </p>
                            </>
                        }
                    }
                    None => {
                        html! { { "Keine Antwort gegeben." }}
                    }
                };
                let hint_line = |(author, hint): (&String, &game::Hint)| {
                    let content = if hint.allowed {
                        html! {
                            { hint.content.clone() }
                        }
                    } else {
                        html! {
                            <s> {hint.content.clone()} </s>
                        }
                    };
                    html! {
                            <tr>
                                <td>{author}</td>
                                <td> { content } </td>
                             </tr>

                    }
                };
                let all_hints = html! {
                    <table>
                        <thead>
                            <tr>
                                <th colspan="2">{"Hinweise"}</th>
                            </tr>
                        </thead>
                        <tbody>
                            { for judging.hints.iter().map(hint_line) }
                        </tbody>
                    </table>
                };
                html! {
                    <>
                    { word }
                    { guess }
                    { all_hints }
                    </>
                }
            }
        };

        if self.ws.is_none() {
            html! {
                <div>
                <button onclick=self.link.callback(|_| WsAction::Connect)>
                { "Verbinden" }
                </button>
                </div>
            }
        } else {
            // let state = format!("{:#?}", self.state);
            html! {
                <div>
                    // <p>
                    //     { format!("{:#?}", state) }
                    // </p>
                    <p>
                        { prelude }
                    </p>
                    <p>
                        { action_html }
                    </p>
                    <p>
                        { state_html }
                    </p>
                    <p>
                        { past_rounds_html }
                    </p>
                </div>
            }
        }
    }
}

#[wasm_bindgen(start)]
pub fn run_app() {
    App::<Model>::new().mount_to_body();
}
