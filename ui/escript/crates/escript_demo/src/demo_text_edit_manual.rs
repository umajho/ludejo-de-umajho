use std::{cell::RefCell, rc::Rc, vec};

use crate::machine::{
    EguiTextEditOptions, Instruction, Instructor, InstructorNextArgument, Machine,
    NewMachineOptions,
};

pub struct TextEditDemoManual {
    machine: Machine,
}

impl Default for TextEditDemoManual {
    fn default() -> Self {
        let instructor = Box::new(ManualInstructor::new());

        let persisted = Rc::new(RefCell::new(String::new()));

        let machine = Machine::new(
            instructor,
            NewMachineOptions {
                get_persisted: {
                    let persisted = Rc::clone(&persisted);
                    Box::new(move || Some(persisted.borrow().clone()))
                },
                insert_persisted: {
                    let persisted = Rc::clone(&persisted);
                    Box::new(move |data: String| {
                        *persisted.borrow_mut() = data;
                    })
                },
            },
        );

        Self { machine }
    }
}

impl crate::Demo for TextEditDemoManual {
    fn name(&self) -> &'static str {
        "TextEdit (machine - manual)"
    }

    fn ui(&mut self, ui: &mut egui::Ui) {
        self.machine.run(ui);
    }
}

pub struct ManualInstructor {
    state: State,
}

#[derive(derive_more::Debug, Default)]
enum State {
    #[default]
    S0,
    S1 {
        persisted: Persisted,
    },
    S2 {
        persisted: Persisted,
    },
    S3 {
        persisted: Persisted,
    },
    S4 {
        persisted: Persisted,
        #[debug(skip)]
        output: egui::text_edit::TextEditOutput,
    },
    S5 {
        persisted: Persisted,
        #[debug(skip)]
        output: egui::text_edit::TextEditOutput,
    },
    S6 {
        persisted: Persisted,
        #[debug(skip)]
        output: egui::text_edit::TextEditOutput,
    },
    S7 {
        persisted: Persisted,
        #[debug(skip)]
        output: egui::text_edit::TextEditOutput,
    },
    S8 {
        persisted: Persisted,
        #[debug(skip)]
        output: egui::text_edit::TextEditOutput,
        input_1: bool,
    },
    S9 {
        persisted: Persisted,
        #[debug(skip)]
        output: egui::text_edit::TextEditOutput,
    },
    S10 {
        persisted: Persisted,
        #[debug(skip)]
        output: egui::text_edit::TextEditOutput,
    },
    S11 {
        persisted: Persisted,
        #[debug(skip)]
        output: egui::text_edit::TextEditOutput,
    },
    S12 {
        persisted: Persisted,
    },

    Invalid,
}

#[derive(Debug, Default, Clone, serde::Serialize, serde::Deserialize)]
struct Persisted {
    text: String,
}

impl ManualInstructor {
    pub fn new() -> Self {
        Self {
            state: State::default(),
        }
    }
}

impl Instructor for ManualInstructor {
    fn reset(&mut self) {
        self.state = State::default();
    }

    fn next(
        &mut self,
        agent: &mut crate::machine::MachineAgent,
        arg: InstructorNextArgument,
    ) -> Vec<crate::machine::Instruction> {
        let mut is = vec![];

        self.state = match core::mem::replace(&mut self.state, State::Invalid) {
            State::S0 => {
                let mut persisted: Persisted = agent
                    .get_persisted()
                    .and_then(|s| serde_json::from_str(&s).ok())
                    .unwrap_or_default();

                if persisted.text.is_empty() {
                    persisted.text = "Edit this text".into();
                }
                is.push(Instruction::UiHorizontalEnter);

                State::S1 { persisted }
            }
            State::S1 { persisted } => {
                agent.ui().style_mut().spacing.item_spacing.x = 0.0;
                is.push(Instruction::EguiLabel("Advanced usage of ".into()));
                is.push(Instruction::EguiLabel(
                    egui::RichText::new("TextEdit").code().into(),
                ));
                is.push(Instruction::EguiLabel(".".into()));
                is.push(Instruction::Exit);

                State::S2 { persisted }
            }
            State::S2 { persisted } => {
                is.push(Instruction::EguiTextEditMultiline(
                    persisted.text.clone(),
                    EguiTextEditOptions {
                        hint_text: Some("Type something!".into()),
                    },
                ));

                State::S3 { persisted }
            }

            State::S3 { mut persisted } => {
                let (text, output) = arg.into_text_edit_output().unwrap();

                persisted.text = text;

                is.push(Instruction::UiHorizontalEnter);

                State::S4 { persisted, output }
            }
            State::S4 { persisted, output } => {
                agent.ui().style_mut().spacing.item_spacing.x = 0.0;
                is.push(Instruction::EguiLabel("Selected text: ".into()));
                if let Some(text_cursor_range) = output.cursor_range {
                    let selected_text = text_cursor_range.slice_str(&persisted.text);
                    is.push(Instruction::EguiLabel(
                        egui::RichText::new(selected_text).code().into(),
                    ));
                }
                is.push(Instruction::Exit);

                State::S5 { persisted, output }
            }
            State::S5 { persisted, output } => {
                let anything_selected =
                    output.cursor_range.is_some_and(|cursor| !cursor.is_empty());
                is.push(Instruction::UiAddEnabledUiEnter(anything_selected));

                State::S6 { persisted, output }
            }
            State::S6 { persisted, output } => {
                is.push(Instruction::EguiLabel(
                    "Press ctrl+Y to toggle the case of selected text (cmd+Y on Mac)".into(),
                ));
                is.push(Instruction::Exit);

                State::S7 { persisted, output }
            }
            State::S7 { persisted, output } => {
                let has_focus = output.response.has_focus();

                State::S8 {
                    persisted,
                    output,
                    input_1: has_focus
                        && agent
                            .ui()
                            .input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::Y)),
                }
            }
            State::S8 {
                mut persisted,
                output,
                input_1,
            } => {
                if input_1 && let Some(text_cursor_range) = output.cursor_range {
                    use egui::TextBuffer as _;
                    let selected_chars = text_cursor_range.as_sorted_char_range();
                    let selected_text = persisted.text.char_range(selected_chars.clone());
                    let upper_case = selected_text.to_uppercase();
                    let new_text = if selected_text == upper_case {
                        selected_text.to_lowercase()
                    } else {
                        upper_case
                    };
                    persisted.text.delete_char_range(selected_chars.clone());
                    persisted.text.insert_text(&new_text, selected_chars.start);
                }

                is.push(Instruction::UiHorizontalEnter);

                State::S9 { persisted, output }
            }
            State::S9 { persisted, output } => {
                is.push(Instruction::EguiLabel("Move cursor to the:".into()));

                is.push(Instruction::EguiButton("start".into()));

                State::S10 { persisted, output }
            }
            State::S10 { persisted, output } => {
                let response = arg.into_response().unwrap();

                if response.clicked() {
                    let text_edit_id = output.response.id;
                    if let Some(mut state) = egui::TextEdit::load_state(agent.ctx(), text_edit_id) {
                        let ccursor = egui::text::CCursor::new(0);
                        state
                            .cursor
                            .set_char_range(Some(egui::text::CCursorRange::one(ccursor)));
                        egui::TextEdit::store_state(agent.ctx(), text_edit_id, state);
                        agent
                            .ctx()
                            .memory_mut(|mem| mem.request_focus(text_edit_id));
                    }
                }

                is.push(Instruction::EguiButton("end".into()));

                State::S11 { persisted, output }
            }
            State::S11 { persisted, output } => {
                let response = arg.into_response().unwrap();

                if response.clicked() {
                    let text_edit_id = output.response.id;
                    if let Some(mut state) = egui::TextEdit::load_state(agent.ctx(), text_edit_id) {
                        let ccursor = egui::text::CCursor::new(persisted.text.chars().count());
                        state
                            .cursor
                            .set_char_range(Some(egui::text::CCursorRange::one(ccursor)));
                        egui::TextEdit::store_state(agent.ctx(), text_edit_id, state);
                        agent
                            .ctx()
                            .memory_mut(|mem| mem.request_focus(text_edit_id));
                    }
                }

                is.push(Instruction::Exit);

                State::S12 { persisted }
            }
            State::S12 { persisted } => {
                is.push(Instruction::Stop);

                agent.insert_persisted(serde_json::to_string(&persisted).unwrap());

                State::Invalid
            }
            State::Invalid => unreachable!(),
        };

        is
    }
}
