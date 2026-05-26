use rand::seq::SliceRandom;

pub fn shuffled<T: Clone>(items: &[T]) -> Vec<T> {
    let mut out = items.to_vec();
    out.shuffle(&mut rand::thread_rng());
    out
}
