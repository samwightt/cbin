fn main() {
    divan::main();
}

#[divan::bench]
fn convert_pgn_with_file_io() {
    use chessb::{converter::Converter, serializer::Serializer};
    use std::fs::File;
    
    let file = File::open("games.pgn").unwrap();
    let mut serializer = Serializer::new(std::io::sink());
    let mut converter = Converter::new(file, serializer);
    
    while converter.next_game().unwrap_or(false) {}
}

#[divan::bench]
fn convert_pgn_without_file_io(bencher: divan::Bencher) {
    use chessb::{converter::Converter, serializer::Serializer};
    use std::fs;
    
    bencher
        .with_inputs(|| {
            // Setup: read file once
            fs::read_to_string("games.pgn").unwrap()
        })
        .bench_values(|pgn_data| {
            // Benchmark: just the conversion
            let mut serializer = Serializer::new(std::io::sink());
            let mut converter = Converter::new(pgn_data.as_bytes(), serializer);
            
            while converter.next_game().unwrap_or(false) {}
        });
}

#[divan::bench]
fn pgn_reader_baseline(bencher: divan::Bencher) {
    use pgn_reader::{Reader, Visitor};
    use std::fs;
    
    struct NoOpVisitor;
    
    impl Visitor for NoOpVisitor {
        type Tags = ();
        type Movetext = ();
        type Output = ();
        
        fn begin_tags(&mut self) -> std::ops::ControlFlow<Self::Output, Self::Tags> {
            std::ops::ControlFlow::Continue(())
        }
        
        fn begin_movetext(&mut self, _tags: Self::Tags) -> std::ops::ControlFlow<Self::Output, Self::Movetext> {
            std::ops::ControlFlow::Continue(())
        }
        
        fn end_game(&mut self, _movetext: Self::Movetext) -> Self::Output {}
    }
    
    bencher
        .with_inputs(|| {
            fs::read_to_string("games.pgn").unwrap()
        })
        .bench_values(|pgn_data| {
            let mut reader = Reader::new(pgn_data.as_bytes());
            let mut visitor = NoOpVisitor;
            
            while reader.read_game(&mut visitor).unwrap().is_some() {}
        });
}
