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

enum Msg {
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
                            "ws://localhost:9001/websocket",
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
        let to_html = |action: &common::game::Action| {
            let guess = match &self.state.phase {
                game::VisibleGamePhase::Judging(game::VisibleJudging::Active(judging))
                | game::VisibleGamePhase::Judging(game::VisibleJudging::Inactive(
                    game::InactiveJudging { judging, .. },
                )) => judging.guess.clone(),
                _ => None,
            };
            match action {
                common::game::Action::Join(id, _) => {
                    let cloned_id: usize = *id;
                    let send_name = move |e: ChangeData| match e {
                        ChangeData::Value(value) => {
                            if !value.trim().is_empty() {
                                Msg::WsSend(common::game::Action::Join(cloned_id, value))
                            } else {
                                Msg::Ignore
                            }
                        }
                        _ => Msg::Ignore,
                    };
                    html! {
                            <div>
                                <label for="uname">{ "Mein Name: " }</label>
                                <input type="text" id="uname" name="name" autocomplete="off" onchange=self.link.callback(send_name)/>
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
                            <button onclick=self.link.callback(send_start) class="button actionbutton startbutton">
                            {"Start"}
                            </button>
                        </div>
                    }
                }
                common::game::Action::GiveHint(id, _hint) => {
                    let cloned_id: usize = *id;
                    let send_hint = move |e: ChangeData| match e {
                        ChangeData::Value(value) => {
                            let trimmed = value.trim();
                            let hint = if trimmed.is_empty() {
                                None
                            } else {
                                Some(trimmed.to_string())
                            };
                            Msg::WsSend(common::game::Action::GiveHint(cloned_id, hint))
                        }
                        _ => Msg::Ignore,
                    };
                    html! {
                        <div>
                            <label for="hint">{ "Hinweis: " }</label>
                            <input type="text" id="hint" name="hint" autocomplete="off" onchange=self.link.callback(send_hint)/>
                        </div>
                    }
                }
                common::game::Action::FilterHint(id, hint, change_validity_to) => {
                    // log::info!("to_html on FilterHint: {:?}, {:?}, {:?}", id, hint, valid);
                    let flip_hint = {
                        let id = *id;
                        let hint = hint.clone();
                        let change_validity_to = *change_validity_to;
                        move |_: yew::ChangeData| {
                            // log::info!(
                            //     "id_loc: {}, hint_loc: {}, valid_loc: {}",
                            //     id_loc,
                            //     hint_loc,
                            //     valid_loc
                            // );
                            Msg::WsSend(common::game::Action::FilterHint(
                                id,
                                hint.clone(),
                                change_validity_to,
                            ))
                        }
                    };
                    let is_valid = !change_validity_to;
                    let hintlabelclass = if is_valid {
                        "hintlabel hintlabel_valid"
                    } else {
                        "hintlabel hintlabel_invalid"
                    };
                    let valid_id = format!("{}_valid", hint);
                    let invalid_id = format!("{}_invalid", hint);
                    let radio = html! {
                        <>
                            <input type="checkbox" class="pseudoradio" id={valid_id.clone()}
                                name={hint} checked=is_valid disabled=is_valid
                                style="padding-left: 5px;"
                                onchange=self.link.callback(flip_hint.clone())/> <label for={valid_id.clone()}>{" gültig "}</label>
                            <input type="checkbox" class="pseudoradio" id={invalid_id.clone()}
                                name={hint} checked=!is_valid disabled=!is_valid
                                onchange=self.link.callback(flip_hint.clone())/> <label for={invalid_id.clone()}>{" ungültig "}</label>
                        </>
                    };
                    html! {
                        <div class="hintline">
                            <div class={hintlabelclass}>
                            {hint}
                            </div>
                            <div>
                            {radio}
                            </div>
                        </div>
                    }
                }
                common::game::Action::FinishHintFiltering(id) => {
                    let id = *id;
                    let send_finish_filtering =
                        move |_| Msg::WsSend(common::game::Action::FinishHintFiltering(id));
                    html! {
                        <div>
                            <button onclick=self.link.callback(send_finish_filtering) class="button actionbutton proceedbutton">
                            {"Hinweisbeurteilung abschliessen"}
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
                            <label for="hint">{ "Ich rate: " }</label>
                            <input type="text" id="guess" name="guess" autocomplete="off" onchange=self.link.callback(send_guess)/>
                        </div>
                        <div style="padding-top: 5px">
                            <button onclick=self.link.callback(send_no_guess) class="button actionbutton guessbutton">
                            {"Keine Ahnung"}
                            </button>
                        </div>
                        </>
                    }
                }
                common::game::Action::Judge(id, change_correct_to) => match guess {
                    Some(guess) => {
                        let flip_judgement = {
                            let change_correct_to = *change_correct_to;
                            let id = *id;
                            move |_: yew::ChangeData| {
                                Msg::WsSend(common::game::Action::Judge(id, change_correct_to))
                            }
                        };
                        let is_correct = !change_correct_to;
                        let hintlabelclass = if is_correct {
                            "hintlabel hintlabel_valid"
                        } else {
                            "hintlabel hintlabel_invalid"
                        };
                        let radio = html! {
                            <>
                                <input type="checkbox" class="pseudoradio" id="valid"
                                    name={guess.clone()} checked=is_correct disabled=is_correct
                                    onchange=self.link.callback(flip_judgement)/>
                                <label for="valid" style="padding-right: 5px;">{" richtig"}</label>
                                <input type="checkbox" class="pseudoradio" id="invalid"
                                    name={guess.clone()} checked=!is_correct disabled=!is_correct
                                    onchange=self.link.callback(flip_judgement)/>
                                <label for="invalid">{" falsch"}</label>
                            </>
                        };
                        html! {
                            <div class="hintline">
                                <div class={hintlabelclass}>
                                {guess}
                                </div>
                                <div>
                                {radio}
                                </div>
                            </div>
                        }
                    }
                    None => {
                        eprintln!("judging action but there is no guess");
                        html! {
                            <></>
                        }
                    }
                },
                common::game::Action::FinishJudging(id) => {
                    let id = *id;
                    let send_finish_judging =
                        move |_| Msg::WsSend(common::game::Action::FinishJudging(id));
                    let noguess = match guess {
                        Some(_) => html! {<></>},
                        None => {
                            html! {
                                <div class="hintline">
                                    {"Es wurde nicht geraten."}
                                </div>
                            }
                        }
                    };
                    html! {
                        <>
                        { noguess }
                        <div>
                            <button onclick=self.link.callback(send_finish_judging) class="button actionbutton finishjudgingbutton">
                            {"Runde abschliessen"}
                            </button>
                        </div>
                        </>
                    }
                }
                common::game::Action::Leave(id) => {
                    let id = *id;
                    let send_leave = move |_| Msg::WsSend(common::game::Action::Leave(id));
                    html! {
                        <div>
                            <button onclick=self.link.callback(send_leave) class="button leavebutton">
                            { "Spiel verlassen" }
                            </button>
                        </div>
                    }
                }
            }
        };

        let action_html = self
            .state
            .actions
            .iter()
            .filter(|a| !matches!(a, game::Action::Leave(_)))
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
            html! {
                    <>
                        { list_players }
                    </>
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
            game::VisibleGamePhase::GatherPlayers => match self.state.me {
                Some(_) => html! { { "Das Spiel hat noch nicht angefangen." } },
                None => html! { { "Noch nicht angemeldet." }},
            },
            game::VisibleGamePhase::HintCollection(game::VisibleHintCollection::Active(
                hint_collection,
            )) => {
                let message = if hint_collection.players_done.len() > 1 {
                    format!(
                        "Es sind schon {} Hinweise eingegangen.",
                        hint_collection.players_done.len()
                    )
                } else if hint_collection.players_done.len() == 1 {
                    format!("Es ist schon 1 Hinweis eingegangen.")
                } else {
                    format!("Es sind noch keine Hinweise eingegangen.")
                };
                html! {
                    { message }
                }
            }
            game::VisibleGamePhase::HintCollection(game::VisibleHintCollection::Inactive(
                hint_collection,
            )) => match &hint_collection.hint {
                Some(hint) => {
                    html! {
                        { format!("Möchtest du deinen Hinweis \"{}\" für \"{}\" ändern?", hint.content, hint_collection.word) }
                    }
                }
                None => {
                    html! { { format!(
                        "Bitte gib {} einen Hinweis für \"{}\".",
                        hint_collection.active_player,
                        hint_collection.word)
                    }}
                }
            },
            game::VisibleGamePhase::HintFiltering(game::VisibleHintFiltering::Active(
                hint_filtering,
            )) => html! { { format!(
                "Es werden doppelte Hinweise entfernt, aktuell sind {} übrig.",
                hint_filtering.players_valid_hints.len()
            ) } },
            game::VisibleGamePhase::HintFiltering(game::VisibleHintFiltering::Inactive(
                game::InactiveHintFiltering { hint_filtering, .. },
            )) => {
                html! { { format!("Welche Hinweise für \"{}\" sind gültig?", hint_filtering.word) } }
            }
            game::VisibleGamePhase::Guessing(game::VisibleGuessing::Active(guessing)) => {
                let render_hint = |hint: &game::VisibleHint| {
                    html! {
                        <li>
                             { hint.clone() }
                        </li>
                    }
                };
                html! {
                    <>
                    { "Die Hinweise sind:" }
                    <ul class="item-list">
                        { for self.state.players.iter().filter_map(|p| guessing.hints.get(&p.name)).map(|h| render_hint(h)  ) }
                    </ul>
                    { "Welches Wort ist gesucht?" }
                    </>
                }
            }
            game::VisibleGamePhase::Guessing(game::VisibleGuessing::Inactive(j)) => {
                html! { { format!("Wir warten bis {} geraten hat.", j.active_player) } }
            }
            game::VisibleGamePhase::Judging(game::VisibleJudging::Active(judging))
            | game::VisibleGamePhase::Judging(game::VisibleJudging::Inactive(
                game::InactiveJudging { judging, .. },
            )) => {
                let word = html! {
                    <p>
                        {format!("Das gesuchte Wort war \"{}\".", judging.word)}
                    </p>
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
                    <table class="allhints">
                        <thead>
                            <tr>
                                <th colspan="2">{"Hinweise"}</th>
                            </tr>
                        </thead>
                        <tbody>
                            { for itertools::sorted(judging.hints.iter()).map(hint_line) }
                        </tbody>
                    </table>
                };
                html! {
                    <>
                    { word }
                    { all_hints }
                    </>
                }
            }
        };

        if self.ws.is_none() {
            html! {
                <div>
                <button onclick=self.link.callback(|_| WsAction::Connect) class="button connectbutton">
                { "Verbinden" }
                </button>
                </div>
            }
        } else {
            let leave = self
                .state
                .actions
                .iter()
                .filter(|a| matches!(a, game::Action::Leave(_)))
                .map(|a| to_html(a))
                .next()
                .unwrap_or(html! {});
            html! {
                <div class="main">
                    // <p>
                    //     { format!("{:#?}", state) }
                    // </p>
                    <div class="prelude">
                        { prelude }
                    </div>
                    <div class="action">
                        { action_html }
                    </div>
                    <div class="state">
                        { state_html }
                    </div>
                    <div class="history">
                        { past_rounds_html }
                    </div>
                    <div class="leave">
                        { leave }
                    </div>
                </div>
            }
        }
    }
}

#[wasm_bindgen(start)]
pub fn run_app() {
    wasm_logger::init(wasm_logger::Config::default());
    App::<Model>::new().mount_to_body();
}
