pub fn take<T>(mut vec: Vec<T>) -> Option<T> {
    if !vec.is_empty() {
        Some(vec.remove(0))
    } else {
        None
    }
}
