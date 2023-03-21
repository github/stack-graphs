/*--- path: Importer.java ---*/
public class Importer {
  public static void main(String[] args) {
    bar();
 // ^ defined:
  }
}

/* --- path: Foo.java ---*/
public class Foo {
  public static void bar() {
  }
}
