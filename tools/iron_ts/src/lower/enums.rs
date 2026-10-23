// SPDX-License-Identifier: Apache-2.0

//! Enums. A numeric enum is a `number` whose members are constants; a string enum is a union of
//! string tags (`Ty::Lit`). Members are folded into their uses, so an enum costs nothing at run time.

use super::expr::{mk, num};
use super::*;

impl<'a> Lowerer<'a> {
    /// Registers `enum E { ... }` / `const enum E { ... }`: its type and the value of each member.
    pub(crate) fn declare_enum(&mut self, e: &'a TSEnumDeclaration<'a>) -> R<()> {
        let name = e.id.name.to_string();
        let mut members: Vec<(String, Expr)> = Vec::new();
        let mut next = Some(0.0);
        let mut ty = Ty::Num;
        // Earlier members are visible by their bare name inside later initialisers (`B = A << 1`).
        let saved: HashMap<String, Option<ConstInfo>> = e
            .body
            .members
            .iter()
            .filter_map(|m| member_name(&m.id).ok())
            .map(|n| (n.clone(), self.consts.get(&n).cloned()))
            .collect();
        let result = (|| {
            for m in &e.body.members {
                let mname = member_name(&m.id)?;
                let value = match &m.initializer {
                    Some(init) => {
                        let v = self.lower_expr(init, None)?;
                        match v.kind {
                            ExprKind::Num(n) => {
                                next = Some(n + 1.0);
                                num(n)
                            }
                            ExprKind::Tag(_) => {
                                ty = Ty::Lit;
                                next = None;
                                v
                            }
                            _ => bail!("enum {name}.{mname}: the value must be a constant"),
                        }
                    }
                    None => {
                        let Some(n) = next else {
                            bail!("enum {name}.{mname}: needs an initialiser after a string member")
                        };
                        next = Some(n + 1.0);
                        num(n)
                    }
                };
                self.consts
                    .insert(mname.clone(), ConstInfo::Inline(value.clone()));
                members.push((mname, value));
            }
            Ok(())
        })();
        for (n, old) in saved {
            match old {
                Some(o) => self.consts.insert(n, o),
                None => self.consts.remove(&n),
            };
        }
        result?;
        self.named.insert(name.clone(), ty.clone());
        let table: HashMap<String, Expr> = members.into_iter().collect();
        self.enums.insert(name, table);
        Ok(())
    }

    /// `E.Member` for an enum `E`.
    pub(crate) fn enum_member(&self, enum_name: &str, member: &str) -> Option<R<Expr>> {
        let table = self.enums.get(enum_name)?;
        Some(match table.get(member) {
            Some(v) => Ok(mk(v.kind.clone(), v.ty.clone())),
            None => Err(format!("enum {enum_name} has no member `{member}`")),
        })
    }
}

fn member_name(id: &TSEnumMemberName) -> R<String> {
    match id {
        TSEnumMemberName::Identifier(i) => Ok(i.name.to_string()),
        TSEnumMemberName::String(s) => Ok(s.value.to_string()),
        _ => bail!("computed enum member name"),
    }
}
