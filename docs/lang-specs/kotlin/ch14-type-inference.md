# Chapter 14. Type Inference


## Contents

[14.1. Smart Casts](#kls-14.1)

[14.1.1. Data-Flow Framework](#kls-14.1.1)

[14.2. Local Type Inference](#kls-14.2)

[14.3. Function Signature Type Inference](#kls-14.3)

[14.4. Builder-Style Type Inference](#kls-14.4)

[14.5. TODO()](#kls-14.5)


Kotlin has a concept of *type inference* for compile-time type information, meaning some type information in the code may be omitted, to be inferred by the compiler. There are two kinds of type inference supported by Kotlin.

- Local type inference, for inferring types of expressions locally, in statement/expression scope;
- Function signature type inference, for inferring types of function return values and/or parameters.

Type inference is a type constraint problem, and is usually solved by a type constraint solver. For this reason, type inference is applicable in situations when the type context contains enough information for the type constraint solver to create an optimal constraint system solution w.r.t. type inference problem.

> Note: for the purposes of type inference, an optimal solution is the one which does not contain any free type variables with no explicit constraints on them.

Kotlin also supports flow-sensitive types in the form of smart casts, which have direct effect on type inference. Therefore, we will discuss them first, before talking about type inference itself.


<a id="kls-14.1"></a>
## 14.1. Smart Casts

Kotlin introduces a limited form of flow-sensitive typing called *smart casts*. Flow-sensitive typing means some expressions in the program may introduce changes to the compile-time types of variables. This allows one to avoid unneeded explicit casting of values in cases when their runtime types are guaranteed to conform to the expected compile-time types.

Flow-sensitive typing may be considered a specific instance of traditional data-flow analysis. Therefore, before we discuss it further, we need to establish the data-flow framework, which we will use for smart casts.


<a id="kls-14.1.1"></a>
### 14.1.1. Data-Flow Framework

**Smart cast lattices**

We assume our data-flow analysis is run on a classic control-flow graph (CFG) structure, where most non-trivial expressions and statements are simplified and/or desugared.

Our data-flow domain is a map lattice SmartCastData = Expression -> SmartCastType, where Expression is any Kotlin expression and SmartCastType = Type x Type sublattice is a product lattice of smart cast data-flow facts of the following kind.

- First component describes the type, which an expression definitely **has**
- Second component describes the type, which an expression definitely **does not have**

The sublattice order, join and meet are defined as follows.

*P*\_1 x *N*\_1 ⊑ *P*\_2 x *N*\_2 ⇔ *P*\_1 <: *P*\_2 ∧ *N*\_1 :> *N*\_2

*P*\_1 x *N*\_1 ⊔ *P*\_2 x *N*\_2 = LUB(*P*\_1, *P*\_2) x GLB(*N*\_1, *N*\_2)

*P*\_1 x *N*\_1 ⊓ *P*\_2 x *N*\_2 = GLB(*P*\_1, *P*\_2) x LUB(*N*\_1, *N*\_2)

> Note: a well-informed reader may notice the second component is behaving very similarly to a *negation* type.
>
> (*P*\_1 & ¬*N*\_1) | (*P*\_2 & ¬*N*\_2) ⊑ (*P*\_1 | *P*\_2) & (¬*N*\_1 | ¬*N*\_2)
>
> = (*P*\_1 | *P*\_2) & ¬(*N*\_1 & *N*\_2)
>
> (*P*\_1 & ¬*N*\_1) & (*P*\_2 & ¬*N*\_2) = (*P*\_1 & *P*\_2) & (¬*N*\_1 & ¬*N*\_2)
>
> = (*P*\_1 & *P*\_2) & ¬(*N*\_1 | *N*\_2)
>
> This is as intended, as "type which an expression definitely does not have" is exactly a negation type. In smart casts, as Kotlin type system does not have negation types, we overapproximate them when needed.

**Smart cast transfer functions**

The data-flow information uses the following transfer functions.

⟦`assume(x is T)`⟧(*s*) = *s*[*x* → *s*(*x*) ⊓ (*T* × ⊤)]

⟦`assume(x !is T)`⟧(*s*) = *s*[*x* → *s*(*x*) ⊓ (⊤ × *T*)]

⟦`x as T`⟧(*s*) = *s*[*x* → *s*(*x*) ⊓ (*T* × ⊤)]

⟦`x !as T`⟧(*s*) = *s*[*x* → *s*(*x*) ⊓ (⊤ × *T*)]

⟦`assume(x == null)`⟧(*s*) = *s*[*x* → *s*(*x*) ⊓ (`kotlin.Nothing?` × ⊤)]

⟦`assume(x != null)`⟧(*s*) = *s*[*x* → *s*(*x*) ⊓ (⊤ × `kotlin.Nothing?`)]

⟦`assume(x === null)`⟧(*s*) = *s*[*x* → *s*(*x*) ⊓ (`kotlin.Nothing?` × ⊤)]

⟦`assume(x !== null)`⟧(*s*) = *s*[*x* → *s*(*x*) ⊓ (⊤ × `kotlin.Nothing?`)]

⟦`assume(x == y)`⟧(*s*) = *s*[*x* → *s*(*x*) ⊓ *s*(*y*), *y* → *s*(*y*) ⊓ *s*(*x*)]

⟦`assume(x != y)`⟧(*s*) = *s*[*x* → *s*(*x*) ⊓ *swap*(*isNullable*(*s*(*y*))), *y* → *s*(*y*) ⊓ *swap*(*isNullable*(*s*(*x*)))]

⟦`assume(x === y)`⟧(*s*) = *s*[*x* → *s*(*x*) ⊓ *s*(*y*), *y* → *s*(*y*) ⊓ *s*(*x*)]

⟦`assume(x !== y)`⟧(*s*) = *s*[*x* → *s*(*x*) ⊓ *swap*(*isNullable*(*s*(*y*))), *y* → *s*(*y*) ⊓ *swap*(*isNullable*(*s*(*x*)))]

⟦`x = y`⟧(*s*) = *s*[*x* → *s*(*y*)]

⟦*killDataFlow*(*x*)⟧(*s*) = *s*[*x* → (⊤ × ⊤)]

⟦*l*⟧(*s*) = ⊔ over *p* ∈ *predecessor*(*l*) of ⟦*p*⟧(*s*)

where

*swap*(*P* × *N*) = *N* × *P*

*isNullable*(*s*) = (`kotlin.Nothing?` × ⊤) if *s* ⊑ (`kotlin.Nothing?` × ⊤), otherwise (⊤ × ⊤)

> Important: transfer functions for `==` and `!=` are used only if the corresponding `equals` implementation is known to be equivalent to reference equality (`===`/`!==`). If not, `==` and `!=` do not provide any smart cast information and their transfer functions are effectively identity functions.


<a id="kls-14.2"></a>
## 14.2. Local Type Inference

> TODO(Not yet written)


<a id="kls-14.3"></a>
## 14.3. Function Signature Type Inference

> TODO(Not yet written)


<a id="kls-14.4"></a>
## 14.4. Builder-Style Type Inference

> TODO(Not yet written)


<a id="kls-14.5"></a>
## 14.5. TODO()

> TODO(Not yet written)
