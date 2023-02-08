/*--- path: a.java ---*/
import foo.bar.Baz;

public class Importer {
  public static void main(String[] args) {
    Baz.some_method();
  // ^ defined: 2
  }
}


/* --- path: b.java ---*/

import foo.bar.Baz;

public class AnotherImporter {
  public static void main(String[] args) {
    Baz.some_method();
  }
}
