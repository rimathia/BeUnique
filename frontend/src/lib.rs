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
                        {"Wir sind uns einig."}
                        </button>
                    </div>
                }
            }
            common::game::Action::Guess(id, _) => {
                let id: usize = *id;
                let send_guess = move |e: ChangeData| match e {
                    ChangeData::Value(value) => Msg::WsSend(common::game::Action::Guess(id, value)),
                    _ => Msg::Ignore,
                };
                html! {
                    <div>
                        <label for="hint">{ "Ich rate:" }</label>
                        <input type="text" id="guess" name="guess" onchange=self.link.callback(send_guess)/>
                    </div>
                }
            }
            common::game::Action::Judge(id, correct) => {
                let send_judgement = {
                    let id = *id;
                    let correct = *correct;
                    move |e: ChangeData| match e {
                        ChangeData::Value(_value) => {
                            Msg::WsSend(common::game::Action::Judge(id, !correct))
                        }
                        _ => Msg::Ignore,
                    }
                };
                html! {
                    <div>
                        <input type="checkbox" id="judge" name="judge" checked = {*correct} onchange=self.link.callback(send_judgement)/>
                        <label for="judge">{"Richtig geraten"}</label>
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
                        {"Wir sind uns einig."}
                        </button>
                    </div>
                }
            }
        };

        let state = format!("{:#?}", self.state);
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
            let list_players = if self.state.players.len() > 0 {
                html! {
                    <p>
                        { "Es sind anwesend:" }
                        <ul class="item-list">
                            { for self.state.players.iter().map(|p|{ if !is_me(p) { p.name.clone() } else { p.name.clone() + " (ich)" }}) }
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

        let prelude = match &self.state.phase {
            game::VisibleGamePhase::GatherPlayers => {
                html! { { "Das Spiel hat noch nicht angefangen." } }
            }
            game::VisibleGamePhase::HintCollection(game::VisibleHintCollection::Active(
                hint_collection,
            )) => {
                html! {
                    { format!(
                        "Es sind schon {} Hinweise eingegangen.",
                        hint_collection.players_done.len()
                    ) }
                }
            }
            game::VisibleGamePhase::HintCollection(game::VisibleHintCollection::Inactive(
                hint_collection,
            )) => html! { { format!(
                "Bitte gib einen Hinweis für das Wort {}.",
                hint_collection.word
            ) } },
            game::VisibleGamePhase::HintFiltering(game::VisibleHintFiltering::Active(
                hint_filtering,
            )) => html! { { format!(
                "Es werden doppelte Hinweise entfernt, aktuell sind {} übrig.",
                hint_filtering.players_valid_hints.len()
            ) } },
            game::VisibleGamePhase::HintFiltering(game::VisibleHintFiltering::Inactive(_)) => {
                html! { { format!("Welche Hinweise sind gültig?") } }
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
                html! { { "Jetzt wird geraten." } }
            }
            _ => html! { { "Noch nicht implementiert." } },
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
            html! {
                <div>
                    <p>
                        { state }
                    </p>
                    <p>
                        { prelude }
                    </p>
                    <p>
                        { action_html }
                    </p>
                    <p>
                        { state_html }
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
