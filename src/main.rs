use std::time;
use std::thread;

mod bytecode;
mod capability;
mod compile;
mod ir;
mod parse;
mod vm;

fn main() {
    let source = r#"
        mov r1 1
        mov r2 16
        mov r3 256
        mov r4 4096
        mov sp 4096
    loop:
        mov r0 .
        add r0 60
        push r0
        xor r0 r0
        jmp #rot
        jmp #loop
    rot:
        push r1
        push r2
        push r3
        push r4
        pop r3
        pop r2
        pop r1
        pop r4
        pop r0
        jmp r0
    "#;
    let ir = parse::parse(source).unwrap();

    let bc = compile::compile(ir).unwrap();
    println!("{:#?}", bc);

    let mut machine = vm::Machine::new();
    machine.memory.store_slice(0, bc.as_slice()).unwrap();

    // clear
    // print!("\x1B[2J\x1B[1;1H");
    loop {
        // return to top
        // print!("\x1B[1;1H");
        println!("REGISTERS: ");
        println!("{:#}", machine.reg);

        machine.tick().unwrap();

        thread::sleep(time::Duration::from_millis(100));
    }
}
