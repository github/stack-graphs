/*--- path: Importer.java ---*/
import foo.Foo;

public class Importer {
  public static void main(String[] args) {
    Foo.bar();
     // ^ defined: 15

  }
}

/* --- path: foo/Foo.java ---*/

public class Foo {
  public static void bar() {
  }
}
