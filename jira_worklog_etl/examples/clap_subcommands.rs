use clap::{Args,Parser, Subcommand};

#[derive(Parser)]
#[clap(version = "1.0", author = "Steinar Overbeck Cook <steinar.cook@gmail.com>")]
struct Opts {
    #[clap(subcommand)]
    subcmd: SubCommand,
}

#[derive(Subcommand)]
enum SubCommand {
    Add(Add),
    Multiply(Multiply),
}

#[derive(Args)]
struct Add {
    num1: i32,
    num2: i32,
}

#[derive(Parser)]
struct Multiply {
    num1: i32,
    num2: i32,
}

fn main() {
    let opts: Opts = Opts::parse();

    match opts.subcmd {
        SubCommand::Add(add) => {
            println!("{} + {} = {}", add.num1, add.num2, add.num1 + add.num2);
        }
        SubCommand::Multiply(multiply) => {
            println!("{} * {} = {}", multiply.num1, multiply.num2, multiply.num1 * multiply.num2);
        }
    }
}
