fn main() {
    struct S;
    impl S {
        fn process(&mut self, s: &str) -> String {
            s.to_string()
        }
        fn test(&mut self, body: &[&str]) {
            let _body_code: String = body.iter().map(|s| self.process(s)).collect();
        }
    }
}
