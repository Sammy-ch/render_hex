use rayon::prelude::*;
use std::env;
use std::thread;

use crossbeam::channel::unbounded;

use svg::Document;
use svg::node::element::path::{Command, Data, Position};
use svg::node::element::{Path, Rectangle};

use crate::Operation::{Forward, Home, Noop, TurnLeft, TurnRight};

use crate::Orientation::{East, North, South, West};

const WIDTH: isize = 400;
const HEIGHT: isize = WIDTH;

const HOME_Y: isize = HEIGHT / 2;
const HOME_X: isize = WIDTH / 2;

const STROKE_WIDTH: usize = 5;

enum Work {
    Task((usize, u8)),
    Finished,
}

#[derive(Debug, Clone, Copy)]
enum Orientation {
    North,
    East,
    South,
    West,
}

#[derive(Debug, Clone, Copy)]
enum Operation {
    Forward(isize),
    TurnLeft,
    TurnRight,
    Home,
    Noop(u8),
}

#[derive(Debug)]
struct Artist {
    x: isize,
    y: isize,
    heading: Orientation,
}

impl Artist {
    fn new() -> Artist {
        Artist {
            x: HOME_X,
            y: HOME_Y,
            heading: North,
        }
    }

    fn home(&mut self) {
        self.x = HOME_X;
        self.y = HOME_Y;
    }

    fn forward(&mut self, distance: isize) {
        match self.heading {
            North => self.y += distance,
            South => self.x -= distance,
            West => self.x += distance,
            East => self.x -= distance,
        }
    }

    fn turn_right(&mut self) {
        self.heading = match self.heading {
            North => East,
            South => West,
            West => North,
            East => South,
        }
    }

    fn turn_left(&mut self) {
        self.heading = match self.heading {
            North => West,
            South => East,
            West => South,
            East => North,
        }
    }

    fn wrap(&mut self) {
        if self.x < 0 {
            self.x = HOME_X;
            self.heading = West;
        } else if self.x > WIDTH {
            self.x = HOME_X;
            self.heading = East;
        }

        if self.y < 0 {
            self.y = HOME_Y;
            self.heading = North;
        } else if self.y > HEIGHT {
            self.y = HOME_Y;
            self.heading = South;
        }
    }
}

fn parse_byte(byte: u8) -> Operation {
    match byte {
        b'0' => Home,
        b'1'..=b'9' => {
            let distance = (byte - 0x30) as isize;
            Forward(distance * (HEIGHT / 10))
        }
        b'a' | b'b' | b'c' => TurnLeft,
        b'd' | b'e' | b'f' => TurnRight,
        _ => Noop(byte),
    }
}

fn parse(input: &str) -> Vec<Operation> {
    let n_threads = 2;
    let (todo_tx, todo_rx) = unbounded();
    let (results_tx, results_rx) = unbounded();

    let mut n_bytes = 0;
    for (i, byte) in input.bytes().enumerate() {
        todo_tx.send(Work::Task((i, byte))).unwrap();
        n_bytes += 1;
    }

    for _ in 0..n_threads {
        todo_tx.send(Work::Finished).unwrap();
    }

    for _ in 0..n_threads {
        let todo = todo_rx.clone();
        let results = results_tx.clone();

        thread::spawn(move || {
            loop {
                let task = todo.recv();
                let result = match task {
                    Err(_) => break,
                    Ok(Work::Finished) => break,
                    Ok(Work::Task((i, byte))) => (i, parse_byte(byte)),
                };
                results.send(result).unwrap();
            }
        });
    }

    let mut ops = vec![Noop(0); n_bytes];
    for _ in 0..n_bytes {
        let (i, op) = results_rx.recv().unwrap();
        ops[i] = op;
    }
    ops
}

fn convert(operations: &Vec<Operation>) -> Vec<Command> {
    let mut turtle = Artist::new();

    let mut path_data = Vec::<Command>::with_capacity(operations.len());

    let start_at_home = Command::Move(Position::Absolute, (HOME_X, HOME_Y).into());
    path_data.push(start_at_home);

    for op in operations {
        match *op {
            Forward(distance) => turtle.forward(distance),
            TurnLeft => turtle.turn_left(),
            TurnRight => turtle.turn_right(),
            Home => turtle.home(),
            Noop(byte) => {
                eprintln!("warning: illegal byte encountered: {:?}", byte);
            }
        };

        let path_segment = Command::Line(Position::Absolute, (turtle.x, turtle.y).into());

        path_data.push(path_segment);
        turtle.wrap();
    }

    path_data
}

fn generate_svg(path_data: Vec<Command>) -> Document {
    let background = Rectangle::new()
        .set("x", 0)
        .set("y", 0)
        .set("width", WIDTH)
        .set("height", HEIGHT)
        .set("fill", "#ffffff");

    let border = background
        .clone()
        .set("fiil-opacity", "0.0")
        .set("stroke", "#cccccc")
        .set("stroke-width", 3 * STROKE_WIDTH);

    let sketch = Path::new()
        .set("fill", "none")
        .set("stroke ", "#2f2f2f")
        .set("stroke-width", STROKE_WIDTH)
        .set("stroke-opacity", "0.9")
        .set("d", Data::from(path_data));

    let document = Document::new()
        .set("viewBox", (0, 0, HEIGHT, WIDTH))
        .set("height", HEIGHT)
        .set("width", WIDTH)
        .set("style", "style=\"outline: 5px solid #800000;\"")
        .add(background)
        .add(border)
        .add(sketch);

    document
}

fn main() {
    let args = env::args().collect::<Vec<String>>();
    let input = args.get(1).unwrap();
    let default_filename = format!("{}.svg", input);
    let save_to = args.get(2).unwrap_or(&default_filename);

    let operations = parse(input);
    let path_data = convert(&operations);

    let document = generate_svg(path_data);
    svg::save(save_to, &document).unwrap();
}
