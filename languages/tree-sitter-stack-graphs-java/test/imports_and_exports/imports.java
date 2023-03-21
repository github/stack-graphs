/*--- path: Importer.java ---*/
import foo.Foo;
        // ^ defined: 14

public class Importer {
  public Foo test() {
      // ^ defined: 2, 14
  }
}

/* --- path: foo/Foo.java ---*/
package foo;

public class Foo {
}
