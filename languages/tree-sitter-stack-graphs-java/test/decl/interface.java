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

interface Set<E> {
    public Iterator<E> iterator() {}
}
