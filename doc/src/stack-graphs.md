# Stack Graphs

```math
\gdef\infer#1#2#3{\text{#1}\begin{array}{c}#2\\\hline#3\end{array}}
\gdef\ScopedSym#1#2{#1\!/\!#2}
\gdef\ScopeVar{\bullet}
\gdef\ScopeId#1{s_#1}
\gdef\SymSt{\Chi}%{\vec{a}}
\gdef\EmptySymSt{{\small(\!)}}
\gdef\ScSt{\Sigma}%{\vec{s}}
\gdef\EmptyScSt{\lozenge}
\gdef\Root{\mathsf{ROOT}}
\gdef\Push#1{{}\raisebox{.35pt}{$\scriptsize\downarrow$}#1}
\gdef\Pop#1{{}\raisebox{.65pt}{$\scriptsize\uparrow$}#1}
\gdef\Jump{\mathsf{JUMP}}
\gdef\Drop{\mathsf{DROP}}
\gdef\Edge#1#2{#1 \rightarrow #2}
\gdef\Path#1#2#3{( #1, #2, #3 )}
\gdef\conc{\!\cdot\!}
\gdef\push#1#2{#2\conc#1}
\gdef\alt{\;\mid\;}
```

This document gives a formal description of stack graphs.

## Formalism

### Stack Graphs

```math
\begin{array}{rccl}
    symbol        & x             &    & \\
    scope         & s             &    & \\
    node          & N             & := & s \alt \Push{x} \alt \Push{\ScopedSym{x}{s}} \alt \Pop{x} \alt \Pop{\ScopedSym{x}{\ScopeVar}} \alt \Jump \alt \Drop \\
    edge          & e             & := & \Edge{N}{N} \\
\end{array}
```

A stack graph is a directed graph over nodes $N$.

NB. The nodes in the graph have identity, but this is currently not reflected in the notation.

### Paths

```math
\begin{array}{rccl}
    scoped~symbol & a             & := & x \alt \ScopedSym{x}{\ScSt} \\
    symbol~stack  & \SymSt        & := & \EmptySymSt \alt \push{a}{\SymSt} \\
    scope~stack   & \ScSt         & := & \EmptyScSt \alt \push{s}{\ScSt} \\
    path          & p             & := & \Path{\vec{N}}{\SymSt}{\ScSt} \\
\end{array}
```

The following rules define what are valid paths in the graph.
The notion of a path is rich, it carries context information that determines what possible next steps are allowed for a path.

```math
\infer{Scope}{
   \Path{\vec{N} \conc N_1}{\SymSt}{\ScSt} \quad \Edge{N_1}{s}
}{
   \Path{\vec{N} \conc N_1 \conc s}{\SymSt}{\ScSt}
}
```

```math
\infer{Push}{
   \Path{\vec{N} \conc N_1}{\SymSt}{\ScSt} \quad \Edge{N_1}{\Push{x}}
}{
   \Path{\vec{N} \conc N_1 \conc \Push{x}}{\push{x}{\SymSt}}{\ScSt}
}
\qquad
\infer{Pop}{
   \Path{\vec{N} \conc N_1}{\push{x}{\SymSt}}{\ScSt} \quad \Edge{N_1}{\Pop{x}}
}{
   \Path{\vec{N} \conc N_1 \conc \Pop{x}}{\SymSt}{\ScSt}
}
```

```math
\infer{PushScoped}{
   \Path{\vec{N} \conc N_1}{\SymSt}{\ScSt} \quad \Edge{N_1}{\Push{\ScopedSym{x}{s}}}
}{
   \Path{\vec{N} \conc N_1 \conc \Push{\ScopedSym{x}{s}}}{\push{\ScopedSym{x}{(\push{s}{\ScSt})}}{\SymSt}}{\ScSt}
}
\qquad
\infer{PopScoped}{
   \Path{\vec{N} \conc N_1}{\push{\ScopedSym{x}{\ScSt'}}{\SymSt}}{\ScSt} \quad \Edge{N_1}{\Pop{\ScopedSym{x}{\ScopeVar}}}
}{
   \Path{\vec{N} \conc N_1 \conc \Pop{\ScopedSym{x}{\ScopeVar}}}{\SymSt}{\ScSt'}
}
```

```math
\infer{Jump}{
   \Path{\vec{N} \conc N_1}{\SymSt}{\push{s}{\ScSt}} \quad \Edge{N_1}{\Jump}
}{
   \Path{\vec{N} \conc N_1 \conc \Jump \conc s}{\SymSt}{\ScSt}
}
\qquad
\infer{Drop}{
   \Path{\vec{N} \conc N_1}{\SymSt}{\ScSt} \quad \Edge{N_1}{\Drop}
}{
   \Path{\vec{N} \conc N_1 \conc \Drop}{\SymSt}{\EmptyScSt}
}
```

A path is considered _complete_ if the following holds:

...

### Partial Paths

...

## Algorithms

This section describes various algorithms for stack graphs.

...
