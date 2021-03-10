use std::collections::LinkedList;

pub fn get_at_index<T>(list: &LinkedList<T>, index: usize) -> Option<&T> {
    for (i, el) in list.iter().enumerate() {
        if i == index {
            return Some(el);
        }
    }

    None
}
