use crate::game::ui::windows::actions::WindowManagerAction;

#[derive(Default)]
pub struct WindowManagerContext {
    pub actions: Vec<WindowManagerAction>,
}

impl WindowManagerContext {
    pub fn post_action(&mut self, action: WindowManagerAction) {
        self.actions.push(action);
    }
}
