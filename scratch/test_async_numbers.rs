use corvo_lang;

fn main() {
    let source = r#"
        @nums = [1, 2, 3, 4, 5]
        @sum = 0
        
        @worker = procedure(@item, @s) {
            @s = math.add(@s, @item)
        }
        
        async_browse(@nums, @worker, @item, shared @sum)
        
        sys.echo(@sum)
    "#;

    match corvo_lang::run_source(source) {
        Ok(_) => {}
        Err(e) => {
            eprintln!("Error: {}", e);
        }
    }
}
