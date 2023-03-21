/*--- path: Importer.java ---*/
public class Importer {
  public static void main(String[] args) {
    Foo.bar();
     // ^ defined: 12

  }
}

/* --- path: Foo.java ---*/
public class Foo {
  public static void bar() {
  }
}
