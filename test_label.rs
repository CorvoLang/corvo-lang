fn main() {
    let mut x = 0;
    'label: if x == 0 {
        x = 1;
        break 'label;
    }
}
