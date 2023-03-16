/*--- path: a.java ---*/
import Foo;

public class Importer {
  public static void main(String[] args) {
    x = new Foo.bar();
              // ^ defined: 16
          // ^ defined: 2, 15

  }
}

/* --- path: foo.java ---*/

public class Foo {
  public static void bar() {
  }
}
