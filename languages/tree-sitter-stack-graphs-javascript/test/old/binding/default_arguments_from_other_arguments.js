function foo(x, y = x) {
   return y;
}

foo({ bar: 1 }).bar;
//              ^ defined: 5

foo({},
    { bar: 2 }).bar;
//              ^ defined: 9
