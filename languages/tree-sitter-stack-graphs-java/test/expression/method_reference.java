class A {
  void f() {
      A::f;
    //^ defined: 1
    //   ^ defined: 2
      A::new;
    //^ defined: 1
    A a = new A();
      a::f;
    //^ defined: 8
    //   ^ defined: 2
  }
}
class B extends A {
  void g() {
    super::f;
    //     ^ defined: 2
  }
}
