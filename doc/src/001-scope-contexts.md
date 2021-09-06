# Scope Context for Modeling Nested Scoping

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

```math
\gdef\PushScoped#1#2{\raisebox{.35pt}{${\scriptsize\downarrow}$}#1.#2}
\gdef\PopScoped#1#2{\raisebox{.65pt}{${\scriptsize\uparrow}$}#1.#2}
\gdef\ContextedScope#1#2{#1[#2]}
\gdef\ScSt{\Sigma}
\gdef\ScCtx{\Psi}
\gdef\EmptyScCtx{\blacklozenge}
```

This is a proposal to change the stack graph formalism by replacing scope stacks with scope contexts.

## Problem

The problem is how to model nested lexical scoping.
This pattern appears when modeling function application in lambda calculus, but also when modeling nested generics and type application.

## Formalism Change

Instead of a single scope stack $\ScSt$ in a path, we have a scope context $\ScCtx$, consisting of a stack of symbol stacks.
This context holds all active pops at this point.
The elements in a scope stack $\ScSt$ are scopes with an attached context $\ScCtx$, which are the enclosing pops at the point of the push.
A $\Drop$ drops the top of the scope stack stack, making the directly enclosing pop active again.
A $\Jump$ jumps to the top element of the top scope stack in the context, drops the enclosing context, and restores the enclosing context of the scope that was jumped to.

The scope context represents lexical nesting, while the scope stack represents a call stack.
A $\Jump$ takes us to the caller, i.e., the previous element in the current scope stack. At this point the current context is irrelevant, and the context of the caller is restored.
A $\Drop$ takes us to the enclosing scope. At that point, the current call stack is irrelevant, and the call stack of the enclosing scope is restored.

With this semantics, $\Drop$ behaves as currently, if the scope stack stack $\ScCtx$ is a singleton, but correctly restores any earlier pops that were there.
Therefore, I think it does not require changes to the existing rules.

### Paths

```math
\begin{array}{rccl}
    symbol~stack  & \SymSt & := & \EmptySymSt \alt \push{a}{\SymSt} \\
    scope~stack   & \ScSt  & := & \EmptyScSt  \alt \push{\ContextedScope{s}{\ScCtx}}{\ScSt} \\
    scope~context & \ScCtx & := & \EmptyScCtx \alt \push{\ScSt}{\ScCtx} \\
    path          & p      & := & \Path{\vec{N}}{\SymSt}{\ScCtx} \\
\end{array}
```

The rules for valid paths change to the following:

```math
\infer{PushScoped'}{
   \Path{\vec{N} \conc N_1}{\SymSt}{\push{\ScSt}{\ScCtx}} \quad \Edge{N_1}{\Push{\ScopedSym{x}{s}}}
}{
   \Path{\vec{N} \conc N_1 \conc \Push{\ScopedSym{x}{s}}}{\push{\ScopedSym{x}{(\push{\ContextedScope{s}{\ScCtx}}{\ScSt})}}{\SymSt}}{\push{\ScSt}{\ScCtx}}
}
\qquad
\infer{PopScoped'}{
   \Path{\vec{N} \conc N_1}{\push{\ScopedSym{x}{\ScSt}}{\SymSt}}{\ScCtx} \quad \Edge{N_1}{\Pop{\ScopedSym{x}{\ScopeVar}}}
}{
   \Path{\vec{N} \conc N_1 \conc \Pop{\ScopedSym{x}{\ScopeVar}}}{\SymSt}{\push{\ScSt}{\ScCtx}}
}
```

```math
\infer{Jump'}{
   \Path{\vec{N} \conc N_1}{\SymSt}{\push{(\push{\ContextedScope{s}{\ScCtx'}}{\ScSt})}{\ScCtx}} \quad \Edge{N_1}{\Jump}
}{
   \Path{\vec{N} \conc N_1 \conc \Jump \conc s}{\SymSt}{\push{\ScSt}{\ScCtx'}}
}
\qquad
\infer{Drop'}{
   \Path{\vec{N} \conc N_1}{\SymSt}{\push{\ScSt}{\ScCtx}} \quad \Edge{N_1}{\Drop}
}{
   \Path{\vec{N} \conc N_1 \conc \Drop}{\SymSt}{\ScCtx}
}
```

Additionally, we should consider a rule that allows dropping empty scope contexts (keeping them empty).
This rule ensures that a reference inside a nested scope can resolve to the definition in the surrounding context, even when it is not part of an application.
The alternative, having a path with a drop node, and one without, to the surrounding context, can cause wrong results, because even when there is context, the drop could be ignored.

```math
\infer{DropEmpty'}{
   \Path{\vec{N} \conc N_1}{\SymSt}{\EmptyScCtx} \quad \Edge{N_1}{\Drop}
}{
   \Path{\vec{N} \conc N_1 \conc \Drop}{\SymSt}{\EmptyScCtx}
}
```

## Partial Paths

...

## Algorithms

...

## Migration

Effect on existing specifications:

...

Effect on existing data:

...
