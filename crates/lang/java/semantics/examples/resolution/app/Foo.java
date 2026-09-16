package app;

import library.Base;

class Bar {}

class Foo extends Base {
    Bar target; // Our target: resolves to library.Base.Bar.
}
