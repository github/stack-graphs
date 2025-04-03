function foo<T extends new () => unknown>(x: T) {}
class Bar<_T> {}
foo(Bar<string>);
//  ^TODO defined: 2
