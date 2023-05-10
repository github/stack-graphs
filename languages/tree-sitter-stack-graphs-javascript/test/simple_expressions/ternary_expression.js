let x = 1;

// Flow in

/**/ x ? x : x;
//   ^ defined: 1
//       ^ defined: 1
//           ^ defined: 1

// Flow out

(y = 1) ?
    /**/ (y, z = 1) :
    //    ^ defined: 12   
    /**/ (y, w = 1);
    //    ^ defined: 12   


/**/ y;
//   ^ defined: 12

/**/ z;
//   ^ defined: 13

/**/ w;
//   ^ defined: 15

// Flow around

/**/ x;
//   ^ defined: 1