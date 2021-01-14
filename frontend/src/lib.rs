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
        // let oninput = self
        //     .link
        //     .callback(|e: InputData| Msg::Add(e.value.parse::<i64>().map_or(0, |i| i)));

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
                            <label for="uname">{ "Choose a username:" }</label>
                            <input type="text" id="uname" name="name" onchange=self.link.callback(send_name)/>
                        </div>
                }
            }
            common::game::Action::DisconnectPlayer(_id) => {
                html! {
                    <div>
                        { "Something weird happened, we should never get disconnect as an action here" }
                    </div>
                }
            }
            _ => {
                html! {
                    <div>
                        { "html for actions not yet implemented" }
                    </div>
                }
            } // common::game::Action::GiveHint(_, _) => {}
              // common::game::Action::FilterHint(_, _, _) => {}
              // common::game::Action::FinishHintFiltering(_) => {}
              // common::game::Action::Judge(_, _) => {}
              // common::game::Action::FinishJudging(_) => {}
        };

        let state = format!("{:?}", self.state);
        let action_html = self
            .state
            .actions
            .iter()
            .map(|action| to_html(action))
            .collect::<Vec<Html>>();

        let state_html = match &self.state.me {
            Some(my_player) => {
                html! {
                    { format!("You are player {}.", my_player.name) }
                }
            }
            None => {
                let list_players = if self.state.players.len() > 0 {
                    html! {
                        <p>
                            { "The following players take part in the game:" }
                            <ul class="item-list">
                                { for self.state.players.iter().map(|p|{ p.name.clone() }) }
                            </ul>
                        </p>
                    }
                } else {
                    html! {
                        <p>
                            { "There are no players yet" }
                        </p>
                    }
                };
                html! {
                    <>
                    <p>
                        { "Would you like to join the game?" }
                    </p>
                        { list_players }
                    </>
                }
            }
        };

        if self.ws.is_none() {
            html! {
                <div>
                <button onclick=self.link.callback(|_| WsAction::Connect)>
                { "connect" }
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
                        { state_html }
                    </p>
                    <p>
                        { action_html }
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
