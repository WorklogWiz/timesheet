use std::fs::File;

fn main() {
    let file = File::open("path-to-journal").unwrap();
    let mut rdr = csv::ReaderBuilder::new()
        .delimiter(b';')
        .has_headers(true)
        .from_reader(file);
    let vec = rdr
        .records()
        .filter_map(|e| match e {
            Ok(record) if &record[1] != "315100" => Some(Ok(record)),
            Ok(_) => None,
            Err(e) => Some(Err(e)),
        })
        .collect::<Result<Vec<_>, _>>()
        .unwrap();

    for record in vec {
        println!("{record:?}");
    }
}
