let x = { bar: 0 };

function foo(y = x) {
   return y;
}

foo({ bar: 1 }).bar;
//              ^ defined: 1, 7
