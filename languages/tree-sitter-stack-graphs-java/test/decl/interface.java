interface I {
  void f();
}
class A implements I {}
//                 ^ defined: 1
interface J extends I {}
//                  ^ defined: 1

interface Iterator<E> {
    public E next() {}
    //     ^ defined: 9
}
