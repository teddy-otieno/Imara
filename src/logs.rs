use std::rc::Rc;
use std::collections::HashMap;
use crate::core::Engine;
use crate::ui::ui::{SimpleUIContainer, TextView, ViewPosition, ViewContainer, View, cast_view};

pub trait Logable {
    fn to_string(&self) -> String;
}

//Systems or manager that will to send logs must implement their own log types

pub struct LogManager {
    logs: HashMap<String, Box<dyn Logable>>,
}

impl LogManager {
    pub fn new() -> Self {
        Self { logs: HashMap::new() }
    }

    pub fn add_log(&mut self, (log_name, log_obj): (String, Box<dyn Logable>)) {
        self.logs.insert(log_name, log_obj);
    }

    pub fn update_ui_logs_view(&self, engine_ptr: *mut Engine) {
        let mut eng = unsafe { engine_ptr.as_mut().unwrap() };

        let mut log_view_obj = eng.get_ui_tree().unwrap().find_element("ui_log");
        let log_view = match &mut log_view_obj {
            Some(view) => {
                let view_obj = Rc::get_mut(view).unwrap();
                view_obj.as_any().downcast_mut::<SimpleUIContainer>().unwrap()
            },
            None => panic!(""),
        };

        for (name, item) in &self.logs {
            if let Some(mut view_obj) = log_view.get_element_by_id(name.as_str()) {
                let element: &mut TextView = cast_view(&mut view_obj).unwrap();

                let eng_font_face_ref = unsafe { engine_ptr.as_mut().unwrap() };
                element.set_text(item.to_string(), &eng_font_face_ref.font_face);
                continue;
            }
            let text_view = TextView::new(name.clone().into_boxed_str(), item.to_string(), ViewPosition::zerod(), 1.0, 10);
            log_view.add_child(Box::new(text_view));
        }

    }
}
