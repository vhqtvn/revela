// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use either::Either;
use move_core_types::gas_algebra::{GasQuantity, UnitDiv};
use std::{
    marker::PhantomData,
    ops::{Add, Mul},
};

/***************************************************************************************************
 * Gas Expression & Visitor
 *
 **************************************************************************************************/
/// Trait representing an abstract expression that can be used to calculate some gas amount.
///
/// It carries a type parameter `E`, indicating an environment in which the expression can be
/// evaluated/materialized.
pub trait GasExpression<E> {
    type Unit;

    /// Evaluates the expression within the given environment to a concrete number.
    fn evaluate(&self, feature_version: u64, env: &E) -> GasQuantity<Self::Unit>;

    /// Traverse the expression in post-order using the given visitor.
    /// See [`GasExpressionVisitor`] for details.
    fn visit(&self, visitor: &mut impl GasExpressionVisitor);

    /// Performs a division on the unit of the expression.
    ///
    /// This is sometimes required if you want to multiply an amount by a certain count.
    fn per<U>(self) -> GasPerUnit<Self, U>
    where
        Self: Sized,
    {
        GasPerUnit {
            inner: self,
            phantom: PhantomData,
        }
    }
}

/// An interface for performing post-order traversal of the tree structure of a gas expression.
///
/// Alternatively, one could think that the callbacks are invoked following the Reverse Polish
/// notation of the expression.
///
/// Here are a few examples:
/// - `1 + 2`
///   - `quantity(1)`
///   - `quantity(2)`
///   - `add()`
/// - `A + B * 50`
///   - `gas_param<A>()`
///   - `gas_param<B>()`
///   - `quantity(50)`
///   - `mul()`
///   - `add()`
pub trait GasExpressionVisitor {
    fn add(&mut self);

    fn mul(&mut self);

    fn gas_param<P>(&mut self);

    fn quantity<U>(&mut self, quantity: GasQuantity<U>);

    fn per<U>(&mut self);
}

/***************************************************************************************************
 * Built-in Gas Expressions
 *
 **************************************************************************************************/
/// Representing the addition of two gas expressions.
#[derive(Debug, Clone)]
pub struct GasAdd<L, R> {
    pub left: L,
    pub right: R,
}

/// Representing the multiplication of two gas expressions.
#[derive(Debug, Clone)]
pub struct GasMul<L, R> {
    pub left: L,
    pub right: R,
}

/// Representing a gas expression divided by a particular unit.
/// This is sometimes required for further multiplications.
#[derive(Debug, Clone)]
pub struct GasPerUnit<T, U> {
    pub inner: T,
    phantom: PhantomData<U>,
}

/***************************************************************************************************
 * Gas Expression Impl
 *
 **************************************************************************************************/
// Notation:
//   E | T: U means T is a valid gas expression with unit U under environment E.

// E | T: U
// ---------
// E | &T: U
impl<E, T> GasExpression<E> for &T
where
    T: GasExpression<E>,
{
    type Unit = T::Unit;

    #[inline]
    fn evaluate(&self, feature_version: u64, env: &E) -> GasQuantity<Self::Unit> {
        (*self).evaluate(feature_version, env)
    }

    #[inline]
    fn visit(&self, visitor: &mut impl GasExpressionVisitor) {
        (*self).visit(visitor)
    }
}

// ---------------------
// E | GasQuantity<U>: U
impl<E, U> GasExpression<E> for GasQuantity<U> {
    type Unit = U;

    #[inline]
    fn evaluate(&self, _feature_version: u64, _env: &E) -> GasQuantity<Self::Unit> {
        *self
    }

    #[inline]
    fn visit(&self, visitor: &mut impl GasExpressionVisitor) {
        visitor.quantity(*self)
    }
}

// E | L: U,  E | R: U
// -------------------
//    E | L + R: U
impl<E, L, R, U> GasExpression<E> for GasAdd<L, R>
where
    L: GasExpression<E, Unit = U>,
    R: GasExpression<E, Unit = U>,
{
    type Unit = U;

    #[inline]
    fn evaluate(&self, feature_version: u64, env: &E) -> GasQuantity<Self::Unit> {
        self.left.evaluate(feature_version, env) + self.right.evaluate(feature_version, env)
    }

    #[inline]
    fn visit(&self, visitor: &mut impl GasExpressionVisitor) {
        self.left.visit(visitor);
        self.right.visit(visitor);
        visitor.add();
    }
}

// E | L: UL,  E | R: UR,  O = UL * UR
// -----------------------------------
//           E | L * R: O
impl<E, L, R, UL, UR, O> GasExpression<E> for GasMul<L, R>
where
    L: GasExpression<E, Unit = UL>,
    R: GasExpression<E, Unit = UR>,
    GasQuantity<UL>: Mul<GasQuantity<UR>, Output = GasQuantity<O>>,
{
    type Unit = O;

    #[inline]
    fn evaluate(&self, feature_version: u64, env: &E) -> GasQuantity<Self::Unit> {
        self.left.evaluate(feature_version, env) * self.right.evaluate(feature_version, env)
    }

    #[inline]
    fn visit(&self, visitor: &mut impl GasExpressionVisitor) {
        self.left.visit(visitor);
        self.right.visit(visitor);
        visitor.mul();
    }
}

// E | L: U,  E | R: U
// -------------------
// E | Either<L, R>: U
impl<E, L, R, U> GasExpression<E> for Either<L, R>
where
    L: GasExpression<E, Unit = U>,
    R: GasExpression<E, Unit = U>,
{
    type Unit = U;

    #[inline]
    fn evaluate(&self, feature_version: u64, env: &E) -> GasQuantity<Self::Unit> {
        match self {
            Either::Left(left) => left.evaluate(feature_version, env),
            Either::Right(right) => right.evaluate(feature_version, env),
        }
    }

    #[inline]
    fn visit(&self, visitor: &mut impl GasExpressionVisitor) {
        match self {
            Either::Left(left) => left.visit(visitor),
            Either::Right(right) => right.visit(visitor),
        }
    }
}

//       E | T: U1
// ----------------------
// E | T.per<U2>(): U1/U2
impl<E, T, U1, U2> GasExpression<E> for GasPerUnit<T, U2>
where
    T: GasExpression<E, Unit = U1>,
{
    type Unit = UnitDiv<U1, U2>;

    #[inline]
    fn evaluate(&self, feature_version: u64, env: &E) -> GasQuantity<Self::Unit> {
        self.inner.evaluate(feature_version, env).per()
    }

    #[inline]
    fn visit(&self, visitor: &mut impl GasExpressionVisitor) {
        self.inner.visit(visitor);
        visitor.per::<U2>();
    }
}

/***************************************************************************************************
 * Arithmetic Operations
 *
 **************************************************************************************************/
macro_rules! impl_add_and_mul {
    (<$($tp: ident),*>, $left_ty: ty, $right_ty: ty) => {
        impl<$($tp),*> Add<$right_ty> for $left_ty {
            type Output = GasAdd<Self, $right_ty>;

            #[inline]
            fn add(self, rhs: $right_ty) -> Self::Output {
                GasAdd {
                    left: self,
                    right: rhs,
                }
            }
        }

        impl<$($tp),*> Mul<$right_ty> for $left_ty {
            type Output = GasMul<Self, $right_ty>;

            #[inline]
            fn mul(self, rhs: $right_ty) -> Self::Output {
                GasMul {
                    left: self,
                    right: rhs,
                }
            }
        }
    };
}

impl_add_and_mul!(<L, R, T>, GasAdd<L, R>, T);
impl_add_and_mul!(<L, R, T>, GasMul<L, R>, T);
impl_add_and_mul!(<T, U, X>, GasPerUnit<T, U>, X);

impl_add_and_mul!(<U, L, R>, GasQuantity<U>, GasAdd<L, R>);
impl_add_and_mul!(<U, L, R>, GasQuantity<U>, GasMul<L, R>);
impl_add_and_mul!(<X, T, U>, GasQuantity<X>, GasPerUnit<T, U>);
