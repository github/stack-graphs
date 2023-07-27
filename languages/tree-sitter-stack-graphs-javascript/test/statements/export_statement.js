let x = 1;

// Flow In

export { x };
//       ^ defined: 1
export { x as _ };
//       ^ defined: 1
export let _ = x;
//             ^ defined: 1
export function f() {
    /**/ x;
    //   ^ defined: 1
};
export default function () {
    /**/ x;
    //   ^ defined: 1
};

// Flow Out

export { _ as y };

/**/ y;
//   ^ defined:
// y should not be defined here

// Flow Around

/**/ x;
//   ^ defined: 1