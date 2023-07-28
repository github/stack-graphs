# Performance Benchmark

A set of files, taken from the TypeScript compiler v4.9.5, of different sizes:

```
      15 text files.
      15 unique files.
       0 files ignored.

github.com/AlDanial/cloc v 1.96  T=0.12 s (121.5 files/s, 250225.1 lines/s)
---------------------------------------------------------------------------------------------
File                                                      blank        comment           code
---------------------------------------------------------------------------------------------
typescript_benchmark/types.ts                              1085           1548           6642
typescript_benchmark/nodeFactory.ts                         598            758           5544
typescript_benchmark/es2015.ts                              477           1082           2861
typescript_benchmark/generators.ts                          301            903           1985
typescript_benchmark/module.ts                              173            394           1437
typescript_benchmark/system.ts                              204            546           1195
typescript_benchmark/resolutionCache.ts                     112            101            920
typescript_benchmark/sourcemap.ts                            99             47            614
typescript_benchmark/utilities.ts                            64             92            371
typescript_benchmark/emitNode.ts                             35             87            172
typescript_benchmark/es5.ts                                  10             40             72
typescript_benchmark/node.ts                                 14              4             66
typescript_benchmark/builderPublic.ts                         8            117             56
typescript_benchmark/es2019.ts                                4              1             33
typescript_benchmark/builderStatePublic.ts                    1              0             13
---------------------------------------------------------------------------------------------
SUM:                                                       3185           5720          21981
---------------------------------------------------------------------------------------------
```
