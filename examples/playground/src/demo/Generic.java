package demo;

import library.Widget;

// Combine class and interface bounds with implemented interfaces.
class Generic<T extends Widget & Labelled & Writable> implements Labelled, Writable {
    // Navigate to the bounded type parameter.
    T value;
    // Hover an array of the type parameter.
    T[] history;

    // Hover an upper bound using the type parameter.
    Box<? extends T> producer;
    // Hover a lower bound using the type parameter.
    Box<? super T> consumer;
}

interface Writable {}
