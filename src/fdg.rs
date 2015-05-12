extern crate rand;

use std::ops::{Add, Sub, Mul, Div, Rem};
use std::io;
use std::str::FromStr;
use std::collections::HashMap;

use std::io::prelude::*;
use std::fs::File;

use std::fmt::{Display, Formatter, Error};

use rand::{Rand, Rng};

use self::Delta::*;

#[derive(Debug,Copy,Clone)]
struct Addr(i64, i64);

#[derive(Debug,Copy,Clone)]
enum Delta { N, S, E, W }

impl Delta {
    fn from_char(c: char) -> Option<Self> {
        match c {
            '^' => Some(N),
            'v' => Some(S),
            '>' => Some(E),
            '<' => Some(W),
            _   => None
        }
    }
}

impl Rand for Delta {
    fn rand<R: Rng>(rng: &mut R) -> Self {
        match rng.gen_range(1, 4) {
            1 => N, 2 => S, 3 => E, 4 => W,
            _ => unreachable!()
        }
    }
}

impl Add<Delta> for Addr {
    type Output = Addr;
    fn add(self, d: Delta) -> Addr {
        let Addr(x, y) = self;
        match d {
            N => Addr(x, y-1),
            S => Addr(x, y+1),
            E => Addr(x+1, y),
            W => Addr(x-1, y)
        }
    }
}

const WIDTH  : usize = 80;
const HEIGHT : usize = 25;

type SymbolTable = HashMap<String, Addr>;

pub struct VM {
    stack: Vec<i64>,
    mem:   [[i64; WIDTH]; HEIGHT],
    pc:    Addr,
    delta: Delta,
    symbols: SymbolTable
}

impl VM {
    pub fn new() -> Self {
        VM {
            stack: vec![],
            mem:   [[b' ' as i64; WIDTH]; HEIGHT],
            pc:    Addr(0, 0), delta: E,
            symbols: HashMap::new()
        }
    }

    pub fn from_file(f : File) -> Self {
        let mut vm = VM::new();

        let mut i = 0; let mut j = 0;
        let mut sym : Option<String> = None;

        for b in f.bytes() {
            match b {
                Ok(b'\n') => { j += 1; i = 0; sym = None; }
                Ok(c) => {
                    if i >= WIDTH { j += 1; i = 0; sym = None; }

                    let a = Addr(i as i64, j as i64);
                    vm.put(a, c as i64);

                    match c {
                        b'{' if sym.is_none() =>
                            { sym = Some(String::new()); }
                        b'}' if sym.is_some() =>
                            { vm.symbols.insert(sym.take().unwrap(), a + E); }
                        _ => {
                            for name in sym.iter_mut() {
                                name.push(c as char)
                            };
                        }
                    }

                    i += 1;
                }

                Err(e) => { println!("Error loading file: {}", e); break }
            }
        };

        vm
    }

    fn fetch(& self, Addr(i, j): Addr) -> i64 {
        self.mem[(j as usize) % HEIGHT][(i as usize) % WIDTH]
    }

    fn put(&mut self, Addr(i, j): Addr, v : i64) {
        self.mem[(j as usize) % HEIGHT][(i as usize) % WIDTH] = v;
    }

    fn instr(& self) -> char    { self.fetch(self.pc) as u8 as char }
    fn jump(&mut self, a: Addr) { self.pc = a; }
    fn next(& self) -> Addr     { self.pc + self.delta }
    fn step(&mut self)          { let n = self.next(); self.jump(n); }
    fn pop(&mut self) -> i64    { self.stack.pop().unwrap_or(0) }

    #[allow(unused_must_use)]
    fn input() -> String {
        let mut stdin = io::stdin();
        let mut buf = String::new();

        stdin.read_line(&mut buf);

        buf
    }

    pub fn run(&mut self) -> i64 {

        macro_rules! binop {
            ($op: expr) => ({
                let y = self.pop();
                let x = self.pop();
                self.stack.push($op(x, y))
            })
        }

        macro_rules! cond {
            ($truept: expr, $falsept: expr) =>
                (self.delta = if self.pop() == 0 { $falsept } else { $truept })
        }

        macro_rules! gather {
            ($end: expr, $cont: expr) => (gather!($end, $cont, char));
            ($end: expr, $cont: expr, $cast: ty) => ({
                self.step();
                let mut c = self.instr();
                while c != $end {
                    $cont.push(c as $cast);
                    self.step(); c = self.instr();
                }
            })
        }

        'eval: loop {
            let i = self.instr();
            match i {
                '0'...'9' | 'a'... 'f' =>
                    self.stack.push(i.to_digit(16).unwrap() as i64),

                '+' => binop!(Add::add),
                '-' => binop!(Sub::sub),
                '*' => binop!(Mul::mul),
                '/' => binop!(Div::div),
                '%' => binop!(Rem::rem),

                '`' => {
                    let y = self.pop();
                    let x = self.pop();
                    self.stack.push(if x > y { 1 } else { 0 });
                }

                '!' => {
                    let b = self.pop();
                    self.stack.push(if b == 0 { 1 } else { 0 })
                }

                '^' | 'v' | '>' | '<' =>
                    self.delta = Delta::from_char(i).unwrap(),

                '?' => self.delta = rand::random(),
                '#' => self.step(),

                '_' => cond!(W, E),
                '|' => cond!(N, S),

                '"' => gather!('"', self.stack, i64),

                '[' => {
                    let mut name = String::new();
                    gather!(']', name);

                    match self.symbols.get(&name) {
                        Some(&a) => {
                            let Addr(ri, rj) = self.next();
                            self.stack.push(ri);
                            self.stack.push(rj);

                            let Addr(i, j) = a;
                            self.stack.push(i);
                            self.stack.push(j);
                        }
                        None => {
                            println!("No such symbol >> {} <<", name);
                            break 'eval;
                        }
                    }
                }

                'r' => {
                    let Addr(i, j) = self.pc;
                    self.stack.push(i);
                    self.stack.push(j);
                }

                'j' => {
                    let j = self.pop();
                    let i = self.pop();
                    self.jump(Addr(i, j));
                }

                ':' => {
                    let x = self.pop();
                    self.stack.push(x);
                    self.stack.push(x);
                }

                '\\' => {
                    let y = self.pop();
                    let x = self.pop();
                    self.stack.push(x);
                    self.stack.push(y);
                }

                '$' => { self.pop(); }

                '.' => {
                    let x = self.pop();
                    print!("{}", x);
                }

                ',' => {
                    let c = self.pop() as u8 as char;
                    print!("{}", c);
                }

                '&' => {
                    let x = i64::from_str(&VM::input()).unwrap();
                    self.stack.push(x);
                }

                '~' => {
                    let c = VM::input().as_bytes()[0] as i64;
                    self.stack.push(c);
                }

                'p' => {
                    let y = self.pop();
                    let x = self.pop();
                    let v = self.pop();
                    self.put(Addr(x, y), v);
                 }

                'g' => {
                    let y = self.pop();
                    let x = self.pop();
                    let v = self.fetch(Addr(x, y));
                    self.stack.push(v);
                }

                '@' => break 'eval,
                ' ' => {}

                _ => {
                    println!("*** Interrupt ***");
                    println!("Unrecognised Opcode Detected: {} @ {:?}",
                             i, self.pc);
                    'confirm: loop {
                        println!("Proceed? (y/n) ");
                        match VM::input().as_bytes()[0] {
                            b'y' | b'Y' => break 'confirm,
                            b'n' | b'N' => break 'eval,
                            _ => {}
                        }
                    }
                }
            };
            self.step();
        }

        self.pop()
    }
}

impl Display for VM {
    fn fmt(&self, fmter: &mut Formatter) -> Result<(), Error> {
        fmter.write_str(&format!("{:?}, {:?}, {:?}",
                                 self.stack, self.pc,
                                 self.delta))
    }
}
