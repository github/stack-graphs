let x = 1;

// Flow in
{
    /**/ x;
    //   ^ defined: 1
}

// Flow around

/**/ x;
//   ^ defined: 1

// Flow out

{
    let y = 1;
}

/**/ y;
//   ^ defined: 17