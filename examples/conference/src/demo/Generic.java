package demo;

class Generic<T extends Widget & Readable & Writable> implements Readable, Writable {
    T value;
    Box<? extends T> producer;
    Box<? super T> consumer;
    T[] history;
}
