extern crate handle;
extern crate sparce_buffer;

fn main() {
    println!("Hello, world!");
    let mut buffer = sparce_buffer::SparceBuffer::<i32>::new();
    let h1 = buffer.Allocate(1);
    let h2 = buffer.Allocate(2);
    let h3 = buffer.Allocate(3);

    buffer[h1] = buffer[h1] * 20;

    println!("h1:{}={}", h1, buffer[h1]);
    println!("h2:{}={}", h2, buffer[h2]);
    println!("h3:{}={}", h3, buffer[h3]);

    for i in buffer.Iter() {
        println!("{}", i)
    }

    buffer.Free(h1);
    println!("h1 is free");

    for i in buffer.Iter() {
        println!("{}", i)
    }

    let h4 = buffer.Allocate(22);
    println!("h4:{}={}", h4, buffer[h4]);
    for i in buffer.Iter() {
        println!("{}", i)
    }
}
