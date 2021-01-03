pub mod game {
    use serde::{Deserialize, Serialize};

    #[derive(Eq, PartialEq, Serialize, Deserialize, Debug, Clone)]
    pub struct Player {
        id: usize,
        name: String,
    }

    #[derive(Eq, PartialEq, Serialize, Deserialize, Debug)]
    pub struct Turn {
        pub id: usize,
        pub new_value: i64,
    }

    #[derive(Eq, PartialEq, Serialize, Deserialize, Debug)]
    pub enum Action {
        AddPlayer(usize, String),
        Play(Turn),
    }

    #[derive(Serialize, Deserialize, Debug, Default, Clone)]
    pub struct State {
        players: Vec<Player>,
        active_index: Option<usize>,
        value: i64,
    }

    #[derive(Serialize, Deserialize, Debug, Default)]
    pub struct PlayerView {
        pub state: State,
        pub me: Option<Player>,
        pub actions: Vec<Action>,
    }

    impl State {
        pub fn new() -> State {
            State {
                players: Vec::new(),
                active_index: None,
                value: 0,
            }
        }

        pub fn action(&mut self, action: &Action) -> Option<()> {
            match action {
                Action::AddPlayer(new_id, new_name) => self.add_player(*new_id, new_name),
                Action::Play(turn) => self.play(turn),
            }
        }

        pub fn add_player(&mut self, new_id: usize, new_name: &str) -> Option<()> {
            match self.players.iter().position(|p| p.name == new_name) {
                Some(_) => None,
                None => {
                    self.players.push(Player {
                        id: new_id,
                        name: new_name.to_string(),
                    });
                    if self.active_index.is_none() {
                        self.active_index = Some(self.players.len() - 1);
                    }
                    Some(())
                }
            }
        }

        fn player_index(&self, id: usize) -> Option<usize> {
            self.players.iter().position(|p| p.id == id)
        }

        pub fn play(&mut self, turn: &Turn) -> Option<()> {
            let submitter_index = self.player_index(turn.id)?;
            if submitter_index == self.active_index? {
                self.value = turn.new_value;
                self.active_index = Some((submitter_index + 1) % self.players.len());
                Some(())
            } else {
                None
            }
        }

        pub fn list_actions(&self, id: usize) -> Vec<Action> {
            match self.player_index(id) {
                Some(i) => {
                    if i == self.active_index.unwrap_or(0) {
                        vec![Action::Play(Turn {
                            id: id,
                            new_value: 0,
                        })]
                    } else {
                        vec![]
                    }
                }
                None => {
                    vec![Action::AddPlayer(id, String::new())]
                }
            }
        }

        pub fn get_view(&self, id: usize) -> PlayerView {
            match self.player_index(id) {
                Some(index) => PlayerView {
                    state: self.clone(),
                    me: Some(self.players[index].clone()),
                    actions: self.list_actions(id),
                },
                None => PlayerView {
                    state: self.clone(),
                    me: None,
                    actions: self.list_actions(id),
                },
            }
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
