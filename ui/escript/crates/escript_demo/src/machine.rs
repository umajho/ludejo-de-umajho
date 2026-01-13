use std::cell::RefCell;

pub struct Machine {
    pub(self) instructor: RefCell<Box<dyn Instructor>>,

    get_persisted: Box<dyn Fn() -> Option<String>>,
    insert_persisted: RefCell<Box<dyn FnMut(String)>>,

    pub(self) next_arg: RefCell<InstructorNextArgument>,
}

pub struct NewMachineOptions {
    pub get_persisted: Box<dyn Fn() -> Option<String>>,
    pub insert_persisted: Box<dyn Fn(String)>,
}

impl Machine {
    pub fn new(instructor: Box<dyn Instructor>, opts: NewMachineOptions) -> Self {
        Self {
            instructor: RefCell::new(instructor),
            get_persisted: opts.get_persisted,
            insert_persisted: RefCell::new(opts.insert_persisted),
            next_arg: RefCell::new(InstructorNextArgument::None),
        }
    }

    pub fn get_persisted(&self) -> Option<String> {
        (self.get_persisted)()
    }

    pub fn insert_persisted(&self, data: String) {
        (self.insert_persisted.borrow_mut())(data)
    }

    pub fn run(&mut self, ui: &mut egui::Ui) {
        self.instructor.borrow_mut().reset();
        while MachineAgent::new(self, ui, false).run() {}
    }
}

pub struct MachineAgent<'a, 'b> {
    machine: &'a Machine,
    ui: &'b mut egui::Ui,
    has_parent_agent: bool,
}

impl<'a, 'b> MachineAgent<'a, 'b> {
    pub fn new(
        machine: &'a Machine,
        ui: &'b mut egui::Ui,
        has_parent_agent: bool,
    ) -> MachineAgent<'a, 'b> {
        MachineAgent {
            machine,
            ui,
            has_parent_agent,
        }
    }

    pub fn ctx(&self) -> &egui::Context {
        self.ui.ctx()
    }

    pub fn ui(&mut self) -> &mut egui::Ui {
        self.ui
    }

    pub fn get_persisted(&self) -> Option<String> {
        self.machine.get_persisted()
    }

    pub fn insert_persisted(&self, data: String) {
        self.machine.insert_persisted(data)
    }

    /// # Returns
    ///
    /// whether should continue running.
    fn run(&mut self) -> bool {
        loop {
            let (is, last) = {
                let mut instructor = self.machine.instructor.borrow_mut();
                let mut is = instructor.next(self, self.machine.next_arg.borrow_mut().take());
                let Some(last) = is.pop() else {
                    continue;
                };
                (is, last)
            };

            for inst in is {
                let ret = self.run_one(inst, false);
                debug_assert!(ret.is_none());
            }
            if matches!(last, Instruction::Stop) {
                debug_assert!(!self.has_parent_agent);
                return false;
            }
            let (arg, should_break) = self.run_one(last, true).unwrap();
            let mut next_arg = self.machine.next_arg.borrow_mut();
            debug_assert!(next_arg.is_none());
            *next_arg = arg;
            if should_break {
                return true;
            }
        }
    }

    /// # Returns
    ///
    /// The `Option` will be some when `is_last` is true, otherwise it will be
    /// `None`.
    ///
    /// When `is_last` is true, the `Option` contains a tuple of:
    /// 1. The argument to call the next `instructor::next`.
    /// 2. Whether the loop in `run` should be broken.
    fn run_one(
        &mut self,
        inst: Instruction,
        is_last: bool,
    ) -> Option<(InstructorNextArgument, bool)> {
        use egui::Widget;

        match inst {
            Instruction::UiHorizontalEnter => {
                let ret = self.ui.horizontal(|ui| {
                    let mut child_agent = MachineAgent::new(self.machine, ui, true);
                    child_agent.run()
                });
                if is_last {
                    return Some((
                        InstructorNextArgument::InnerResponseBool(ret.response, ret.inner),
                        false,
                    ));
                }
            }
            Instruction::EguiLabel(widget_text) => {
                let ret = egui::Label::new(widget_text).ui(self.ui);
                if is_last {
                    return Some((InstructorNextArgument::Response(ret), false));
                }
            }
            Instruction::Exit => {
                return Some((InstructorNextArgument::None, true));
            }
            Instruction::EguiTextEditMultiline(text, opts) => {
                let mut text = text;
                let w = egui::TextEdit::multiline(&mut text);
                let w = if let Some(hint_text) = opts.hint_text {
                    w.hint_text(hint_text)
                } else {
                    w
                };
                let ret = w.show(self.ui);
                if is_last {
                    return Some((InstructorNextArgument::TextEditOutput(text, ret), false));
                }
            }
            Instruction::UiAddEnabledUiEnter(is_enabled) => {
                let ret = self.ui.add_enabled_ui(is_enabled, |ui| {
                    let mut child_agent = MachineAgent::new(self.machine, ui, true);
                    child_agent.run()
                });
                if is_last {
                    return Some((
                        InstructorNextArgument::InnerResponseBool(ret.response, ret.inner),
                        false,
                    ));
                }
            }
            Instruction::EguiButton(atom) => {
                let ret = self.ui.button(atom);
                if is_last {
                    return Some((InstructorNextArgument::Response(ret), false));
                }
            }
            Instruction::Stop => unreachable!(),
        };

        None
    }
}

#[derive(derive_more::Debug)]
pub enum Instruction {
    UiHorizontalEnter,
    EguiLabel(egui::WidgetText),
    Exit,
    EguiTextEditMultiline(String, EguiTextEditOptions),
    UiAddEnabledUiEnter(bool),
    EguiButton(egui::Atom<'static>),
    Stop,
}

#[derive(derive_more::Debug)]
pub struct EguiTextEditOptions {
    pub hint_text: Option<egui::WidgetText>,
}

pub trait Instructor {
    fn reset(&mut self);
    fn next(&mut self, agent: &mut MachineAgent, arg: InstructorNextArgument) -> Vec<Instruction>;
}

#[allow(unused)]
pub enum InstructorNextArgument {
    None,
    TextEditOutput(String, egui::text_edit::TextEditOutput),
    Response(egui::Response),
    InnerResponseBool(egui::Response, bool),
}

impl InstructorNextArgument {
    pub fn take(&mut self) -> InstructorNextArgument {
        core::mem::replace(self, InstructorNextArgument::None)
    }

    pub fn is_none(&self) -> bool {
        matches!(self, InstructorNextArgument::None)
    }

    pub fn into_text_edit_output(self) -> Option<(String, egui::text_edit::TextEditOutput)> {
        match self {
            InstructorNextArgument::TextEditOutput(text, output) => Some((text, output)),
            _ => None,
        }
    }

    pub fn into_response(self) -> Option<egui::Response> {
        match self {
            InstructorNextArgument::Response(response) => Some(response),
            _ => None,
        }
    }
}
