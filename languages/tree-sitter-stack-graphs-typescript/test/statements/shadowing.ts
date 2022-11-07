type T = number;

declare let x: T;

class A<T> {
  m(x: T) {
  //   ^ defined: 5
      x;
    //^ defined: 6
  }
}

function f(x) {
    x;
  //^ defined: 13
};

{
  type T = number;
  let x: T;
  //     ^ defined: 19
    x;
  //^ defined: 20
}

function(x) {
    x;
  //^ defined: 26
};

export {};
