pub mod game {
    use serde::{Deserialize, Serialize};
    use std::collections::HashMap;

    #[derive(Eq, PartialEq, Serialize, Deserialize, Debug, Clone)]
    pub struct Player {
        id: Option<usize>,
        pub name: String,
    }

    #[derive(Eq, PartialEq, Serialize, Deserialize, Debug)]
    pub struct Turn {
        pub id: usize,
        pub new_value: i64,
    }

    #[derive(Eq, PartialEq, Serialize, Deserialize, Debug)]
    pub enum Action {
        Join(usize, String),
        DisconnectPlayer(usize),
        Start(usize),
        GiveHint(usize, String),
        FilterHint(usize, String, bool),
        FinishHintFiltering(usize),
        Guess(usize, String),
        Judge(usize, bool),
        FinishJudging(usize),
    }

    #[derive(Serialize, Deserialize, Debug, Default, Clone)]
    pub struct Hint {
        content: String,
        allowed: bool,
    }

    #[derive(Serialize, Deserialize, Debug, Default, Clone)]
    pub struct HintCollection {
        word: String,
        hints: HashMap<String, Hint>,
    }

    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub struct ActiveHintCollection {
        pub players_done: Vec<String>,
    }
    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub struct InactiveHintCollection {
        pub word: String,
        pub hint: Option<Hint>,
        pub players_done: Vec<String>,
    }
    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub enum VisibleHintCollection {
        Active(ActiveHintCollection),
        Inactive(InactiveHintCollection),
    }

    #[derive(Serialize, Deserialize, Debug, Default, Clone)]
    pub struct HintFiltering {
        word: String,
        hints: HashMap<String, Hint>,
    }

    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub struct ActiveHintFiltering {
        pub players_valid_hints: Vec<String>,
    }

    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub struct InactiveHintFiltering(HintFiltering);

    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub enum VisibleHintFiltering {
        Active(ActiveHintFiltering),
        Inactive(InactiveHintFiltering),
    }

    #[derive(Serialize, Deserialize, Debug, Default, Clone)]
    pub struct Guessing {
        word: String,
        hints: HashMap<String, Hint>,
        guess: Option<String>,
    }

    #[derive(Serialize, Deserialize, Debug, Default, Clone)]
    pub struct VisibleHint(pub String);

    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub struct ActiveGuessing {
        pub hints: HashMap<String, VisibleHint>,
        pub guess: Option<String>,
    }

    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub struct InactiveGuessing(Guessing);

    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub enum VisibleGuessing {
        Active(ActiveGuessing),
        Inactive(InactiveGuessing),
    }
    #[derive(Serialize, Deserialize, Debug, Default, Clone)]
    pub struct Judging {
        word: String,
        hints: HashMap<String, Hint>,
        guess: Option<String>,
        success: Option<bool>,
    }

    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub enum VisibleJudging {
        Active(Judging),
        Inactive(Judging),
    }

    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub enum GamePhase {
        GatherPlayers,
        HintCollection(HintCollection),
        HintFiltering(HintFiltering),
        Guessing(Guessing),
        Judging(Judging),
    }

    impl Default for GamePhase {
        fn default() -> Self {
            GamePhase::GatherPlayers
        }
    }

    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub enum VisibleGamePhase {
        GatherPlayers,
        HintCollection(VisibleHintCollection),
        HintFiltering(VisibleHintFiltering),
        Guessing(VisibleGuessing),
        Judging(VisibleJudging),
    }

    impl Default for VisibleGamePhase {
        fn default() -> Self {
            VisibleGamePhase::GatherPlayers
        }
    }

    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub struct Dictionary {
        candidate_words: Vec<String>,
        used_words: Vec<String>,
    }

    impl Default for Dictionary {
        fn default() -> Self {
            Dictionary {
                candidate_words: vec!["Steinbruch".to_string(), "Schwan".to_string()],
                used_words: vec![],
            }
        }
    }

    impl Dictionary {
        pub fn new(words: Vec<String>) -> Self {
            Self {
                candidate_words: words,
                used_words: vec![],
            }
        }
        fn get_word(&mut self) -> String {
            if self.candidate_words.is_empty() {
                std::mem::swap(&mut self.candidate_words, &mut self.used_words);
            }
            let word = self.candidate_words.pop().unwrap();
            self.used_words.push(word.clone());
            word
        }
    }

    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub struct State {
        pub players: Vec<Player>,
        pub active_index: Option<usize>,
        pub phase: GamePhase,
        pub past_rounds: Vec<(Player, String, bool)>,
        pub dictionary: Dictionary,
    }

    #[derive(Serialize, Deserialize, Debug, Default)]
    pub struct PlayerView {
        pub players: Vec<Player>,
        pub me: Option<Player>,
        pub phase: VisibleGamePhase,
        pub actions: Vec<Action>,
    }

    impl State {
        pub fn new(dictionary: Dictionary) -> Self {
            Self {
                players: vec![],
                active_index: None,
                phase: GamePhase::default(),
                past_rounds: vec![],
                dictionary: dictionary,
            }
        }

        pub fn action(&mut self, action: &Action) -> Option<()> {
            match action {
                Action::Join(new_id, new_name) => self.join(*new_id, new_name),
                Action::DisconnectPlayer(id) => self.disconnect_player(*id),
                Action::Start(_id) => self.start(),
                Action::GiveHint(id, hint) => self.process_hint(*id, hint),
                Action::FilterHint(id, hint, valid) => self.process_hint_filter(*id, hint, *valid),
                Action::FinishHintFiltering(id) => self.process_finish_hint_filter(*id),
                Action::Guess(id, guess) => self.process_guess(*id, guess),
                Action::Judge(id, correct) => self.process_guess_judgement(*id, *correct),
                Action::FinishJudging(id) => self.process_finish_judging(*id),
            }
        }

        pub fn disconnect_player(&mut self, disconnect_id: usize) -> Option<()> {
            self.players
                .iter_mut()
                .find(|p| p.id == Some(disconnect_id))?
                .id = None;
            Some(())
        }

        pub fn join(&mut self, new_id: usize, new_name: &str) -> Option<()> {
            match self.players.iter_mut().find(|p| p.name == new_name) {
                Some(player) => {
                    if player.id.is_none() {
                        player.id = Some(new_id);
                    }
                    Some(())
                }
                None => {
                    self.players.push(Player {
                        id: Some(new_id),
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
            self.players.iter().position(|p| p.id == Some(id))
        }

        fn player(&self, id: usize) -> Option<&Player> {
            self.players.iter().find(|p| p.id == Some(id))
        }

        fn start(&mut self) -> Option<()> {
            match &mut self.phase {
                GamePhase::GatherPlayers => {
                    self.phase = GamePhase::HintCollection(HintCollection {
                        word: self.dictionary.get_word(),
                        hints: HashMap::new(),
                    });
                    Some(())
                }
                _ => {
                    eprintln!("start in wrong game state");
                    None
                }
            }
        }

        fn process_hint(&mut self, id: usize, hint: &str) -> Option<()> {
            let submitter_index = self.player_index(id)?;
            let submitter = &self.players[submitter_index];
            match &mut self.phase {
                GamePhase::HintCollection(hint_collection) => {
                    if submitter_index != self.active_index? {
                        hint_collection.hints.insert(
                            submitter.name.clone(),
                            Hint {
                                content: hint.to_string(),
                                allowed: true,
                            },
                        );
                        if hint_collection.hints.len() == self.players.len() - 1 {
                            self.phase = GamePhase::HintFiltering(HintFiltering {
                                word: hint_collection.word.clone(),
                                hints: hint_collection.hints.clone(),
                            });
                        }
                        Some(())
                    } else {
                        eprintln!("hint from active player? {:?}", (id, hint));
                        None
                    }
                }
                _ => {
                    eprintln!("hint in wrong game state");
                    None
                }
            }
        }

        fn process_hint_filter(&mut self, id: usize, hint: &str, allowed: bool) -> Option<()> {
            let active = self.player_index(id)? == self.active_index?;
            match &mut self.phase {
                GamePhase::HintFiltering(HintFiltering { word: _, hints }) => {
                    if !active {
                        for (_author, h) in hints.iter_mut() {
                            if h.content == hint {
                                h.allowed = allowed;
                            }
                        }
                        Some(())
                    } else {
                        eprintln!("hint filtering from active player");
                        None
                    }
                }
                _ => {
                    eprintln!("hint filtering in wrong game state");
                    None
                }
            }
        }

        fn process_finish_hint_filter(&mut self, id: usize) -> Option<()> {
            let active = self.player_index(id)? == self.active_index?;
            match &mut self.phase {
                GamePhase::HintFiltering(HintFiltering { word, hints }) => {
                    if !active {
                        self.phase = GamePhase::Guessing(Guessing {
                            word: word.clone(),
                            hints: hints.clone(),
                            guess: None,
                        });
                        Some(())
                    } else {
                        eprintln!("finish hint filtering from active player");
                        None
                    }
                }
                _ => {
                    eprintln!("finish hint filtering in wrong game state");
                    None
                }
            }
        }

        fn process_guess(&mut self, id: usize, input_guess: &str) -> Option<()> {
            let active = self.player_index(id)? == self.active_index?;
            match &mut self.phase {
                GamePhase::Guessing(Guessing { word, hints, guess }) => {
                    if active {
                        *guess = Some(input_guess.to_string());
                        self.phase = GamePhase::Judging(Judging {
                            word: word.clone(),
                            hints: hints.clone(),
                            guess: guess.clone(),
                            success: None,
                        });
                        Some(())
                    } else {
                        eprintln!("guess from inactive player");
                        None
                    }
                }
                _ => {
                    eprintln!("guess in wrong game state");
                    None
                }
            }
        }

        fn process_guess_judgement(&mut self, id: usize, correct: bool) -> Option<()> {
            let active = self.player_index(id)? == self.active_index?;
            match &mut self.phase {
                GamePhase::Judging(Judging {
                    word: _,
                    hints: _,
                    guess: _,
                    success,
                }) => {
                    if !active {
                        *success = Some(correct);
                        Some(())
                    } else {
                        eprintln!("judgement from active player");
                        None
                    }
                }
                _ => {
                    eprintln!("guess judgement in wrong game state");
                    None
                }
            }
        }

        fn process_finish_judging(&mut self, _id: usize) -> Option<()> {
            match &mut self.phase {
                GamePhase::Judging(Judging {
                    word,
                    hints: _,
                    guess: _,
                    success,
                }) => {
                    let active_player = &self.players[self.active_index?];
                    self.past_rounds.push((
                        active_player.clone(),
                        word.clone(),
                        success.unwrap_or(false),
                    ));
                    self.active_index = Some((self.active_index? + 1) % self.players.len());
                    self.phase = GamePhase::HintCollection(HintCollection {
                        word: self.dictionary.get_word(),
                        hints: HashMap::new(),
                    });
                    Some(())
                }
                _ => {
                    eprintln!("finish judging in wrong game state");
                    None
                }
            }
        }

        pub fn list_actions(&self, id: usize) -> Vec<Action> {
            match self.player_index(id) {
                Some(i) => {
                    let active = i == self.active_index.unwrap_or(0);
                    match &self.phase {
                        GamePhase::GatherPlayers => {
                            if self.players.len() >= 2 {
                                vec![Action::Start(id)]
                            } else {
                                vec![]
                            }
                        }
                        GamePhase::HintCollection(_) => {
                            if !active {
                                vec![Action::GiveHint(id, String::new())]
                            } else {
                                vec![]
                            }
                        }
                        GamePhase::HintFiltering(HintFiltering { word: _, hints }) => {
                            if !active {
                                let mut actions: Vec<Action> = hints
                                    .iter()
                                    .map(|(_, hint)| {
                                        Action::FilterHint(id, hint.content.clone(), hint.allowed)
                                    })
                                    .collect();
                                actions.push(Action::FinishHintFiltering(id));
                                actions
                            } else {
                                vec![]
                            }
                        }
                        GamePhase::Guessing(_) => {
                            if !active {
                                vec![]
                            } else {
                                vec![Action::Guess(id, String::new())]
                            }
                        }
                        GamePhase::Judging(_) => {
                            if !active {
                                vec![Action::Judge(id, true), Action::FinishJudging(id)]
                            } else {
                                vec![]
                            }
                        }
                    }
                }
                None => {
                    vec![Action::Join(id, String::new())]
                }
            }
        }

        pub fn get_view(&self, id: usize) -> PlayerView {
            let visible_phase: VisibleGamePhase = match self.player_index(id) {
                Some(index) => {
                    let active = index == self.active_index.unwrap_or(0);
                    let player = &self.players[index];
                    match &self.phase {
                        GamePhase::GatherPlayers => VisibleGamePhase::GatherPlayers,
                        GamePhase::HintCollection(HintCollection { word, hints }) => {
                            let done = hints.iter().map(|(key, _)| key.to_string()).collect();
                            if active {
                                VisibleGamePhase::HintCollection(VisibleHintCollection::Active(
                                    ActiveHintCollection { players_done: done },
                                ))
                            } else {
                                let own_hint = hints.get(&player.name).cloned();
                                VisibleGamePhase::HintCollection(VisibleHintCollection::Inactive(
                                    InactiveHintCollection {
                                        word: word.clone(),
                                        hint: own_hint,
                                        players_done: done,
                                    },
                                ))
                            }
                        }
                        GamePhase::HintFiltering(HintFiltering { word, hints }) => {
                            if active {
                                VisibleGamePhase::HintFiltering(VisibleHintFiltering::Active(
                                    ActiveHintFiltering {
                                        players_valid_hints: hints
                                            .iter()
                                            .map(|(name, _)| name.to_string())
                                            .collect(),
                                    },
                                ))
                            } else {
                                VisibleGamePhase::HintFiltering(VisibleHintFiltering::Inactive(
                                    InactiveHintFiltering(HintFiltering {
                                        word: word.clone(),
                                        hints: hints.clone(),
                                    }),
                                ))
                            }
                        }
                        GamePhase::Guessing(Guessing { word, hints, guess }) => {
                            if active {
                                VisibleGamePhase::Guessing(VisibleGuessing::Active(
                                    ActiveGuessing {
                                        hints: hints
                                            .iter()
                                            .map(|(key, value)| {
                                                (key.clone(), VisibleHint(value.content.clone()))
                                            })
                                            .collect(),
                                        guess: None,
                                    },
                                ))
                            } else {
                                VisibleGamePhase::Guessing(VisibleGuessing::Inactive(
                                    InactiveGuessing(Guessing {
                                        word: word.clone(),
                                        hints: hints.clone(),
                                        guess: guess.clone(),
                                    }),
                                ))
                            }
                        }
                        GamePhase::Judging(judging) => {
                            if active {
                                VisibleGamePhase::Judging(VisibleJudging::Active(judging.clone()))
                            } else {
                                VisibleGamePhase::Judging(VisibleJudging::Inactive(judging.clone()))
                            }
                        }
                    }
                }
                None => VisibleGamePhase::GatherPlayers,
            };
            let actions = self.list_actions(id);
            PlayerView {
                players: self.players.clone(),
                me: self.player(id).cloned(),
                phase: visible_phase,
                actions: actions,
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
