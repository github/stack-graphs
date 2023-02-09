/*--- path: a.java ---*/
import Foo;


public class Importer {
  public static void main(String[] args) {
    x = new Foo.bar();
              // ^ defined: 17
          // ^ defined: 2, 16

  }
}

/* --- path: foo.java ---*/

public class Foo {
  public static void bar() {
  }
}
