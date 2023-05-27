use tokio_postgres::types::ToSql;


struct Person {
    id: i32,
    name: String,
    children: Vec<Child>,
}

struct Child {
    name: String,
}

fn main() {

    let persons = vec![
        Person {id: 1, name:"Steinar".to_string(), children: vec![Child{name: "Christina".to_string()}, Child{name:" Michael".to_string()}] },
        Person {id: 2, name: "Johanne".to_string(), children: vec![Child{ name:"Mathilde".to_string()}, Child{name:"Mats".to_string()}]}
    ];

    let _params: Vec<_> = persons.iter().flat_map(|row| [&row.id as &(dyn ToSql + Sync ), &row.name]).collect();

    let children: Vec<String> = persons.into_iter().flat_map(|p| p.children).map(|c| c.name).collect();
    assert_eq!(children.len(), 3);
}