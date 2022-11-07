class V {
  value: number = 1;
}

declare let c: new() => V;

new c().value;
//  ^ defined: 5
//      ^ defined: 2

export {};
