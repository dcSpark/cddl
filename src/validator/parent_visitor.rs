#![cfg(feature = "ast-parent")]

use crate::{
  ast::*,
  token::{ByteValue, Value},
  visitor::{self, *},
};

use std::{borrow::Cow, fmt};

/// validation Result
pub type Result = std::result::Result<(), Error>;

/// validation error
#[derive(Debug)]
pub enum Error {
  /// Tree overwrite error
  Overwrite,
}

impl fmt::Display for Error {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    match self {
      Error::Overwrite => write!(f, "attempt to overwrite existing tree node"),
    }
  }
}

impl std::error::Error for Error {
  fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
    match self {
      _ => None,
    }
  }
}

#[derive(Debug, Default, Clone)]
struct ArenaTree<'a, 'b: 'a> {
  arena: Vec<Node<'a, 'b>>,
}

impl<'a, 'b: 'a> ArenaTree<'a, 'b> {
  fn node(&mut self, val: CDDLType<'a, 'b>) -> usize {
    for node in self.arena.iter() {
      if node.val == val {
        return node.idx;
      }
    }

    let idx = self.arena.len();
    self.arena.push(Node::new(idx, val));
    idx
  }
}

#[derive(Debug, Clone)]
struct Node<'a, 'b: 'a> {
  idx: usize,
  val: CDDLType<'a, 'b>,
  parent: Option<usize>,
  children: Vec<usize>,
}

impl<'a, 'b: 'a> Node<'a, 'b> {
  fn new(idx: usize, val: CDDLType<'a, 'b>) -> Self {
    Self {
      idx,
      val,
      parent: None,
      children: vec![],
    }
  }
}

/// validator type
// #[derive(Clone)]
pub struct ParentVisitor<'a, 'b: 'a> {
  cddl: &'a CDDL<'a>,
  arena_tree: ArenaTree<'a, 'b>,
}

impl<'a, 'b: 'a> ParentVisitor<'a, 'b> {
  pub fn new(cddl: &'a CDDL<'a>) -> Self {
    ParentVisitor {
      cddl,
      arena_tree: ArenaTree {
        arena: Vec::default(),
      },
    }
  }
}

impl<'a, 'b: 'a> ParentVisitor<'a, 'b> {
  fn insert(&mut self, parent: usize, child: usize) -> Result {
    match self.arena_tree.arena[child].parent {
      Some(_) => {
        return Err(Error::Overwrite);
      }
      None => {
        self.arena_tree.arena[child].parent = Some(parent);
      }
    }

    self.arena_tree.arena[parent].children.push(child);

    Ok(())
  }
}

impl<'a, 'b: 'a> Visitor<'a, 'b, Error> for ParentVisitor<'a, 'b> {
  fn visit_cddl(&mut self, cddl: &'b CDDL<'a>) -> visitor::Result<Error> {
    let parent = self.arena_tree.node(CDDLType::CDDL(cddl));
    for rule in cddl.rules.iter() {
      let child = self.arena_tree.node(CDDLType::Rule(rule));

      self.insert(parent, child)?;

      self.visit_rule(rule)?;
    }

    Ok(())
  }

  fn visit_rule(&mut self, rule: &'b Rule<'a>) -> visitor::Result<Error> {
    let parent = self.arena_tree.node(CDDLType::Rule(rule));

    match rule {
      Rule::Group { rule, .. } => {
        let child = self.arena_tree.node(CDDLType::GroupRule(rule));

        self.insert(parent, child)?;

        self.visit_group_rule(rule)?;
      }
      Rule::Type { rule, .. } => {
        let child = self.arena_tree.node(CDDLType::TypeRule(rule));

        self.insert(parent, child)?;

        self.visit_type_rule(rule)?;
      }
    }

    Ok(())
  }

  fn visit_type_rule(&mut self, tr: &'b TypeRule<'a>) -> visitor::Result<Error> {
    let parent = self.arena_tree.node(CDDLType::TypeRule(tr));

    let child = self.arena_tree.node(CDDLType::Identifier(&tr.name));
    self.insert(parent, child)?;

    if let Some(params) = &tr.generic_params {
      let child = self.arena_tree.node(CDDLType::GenericParams(params));
      self.insert(parent, child)?;
      walk_generic_params(self, params)?;
    }

    let child = self.arena_tree.node(CDDLType::Type(&tr.value));
    self.insert(parent, child)?;

    self.visit_type(&tr.value)
  }

  fn visit_group_rule(&mut self, gr: &'b GroupRule<'a>) -> visitor::Result<Error> {
    let parent = self.arena_tree.node(CDDLType::GroupRule(gr));

    let child = self.arena_tree.node(CDDLType::Identifier(&gr.name));
    self.insert(parent, child)?;
    self.visit_identifier(&gr.name)?;

    if let Some(params) = &gr.generic_params {
      let child = self.arena_tree.node(CDDLType::GenericParams(params));
      self.insert(parent, child)?;
      walk_generic_params(self, params)?;
    }

    let child = self.arena_tree.node(CDDLType::GroupEntry(&gr.entry));
    self.insert(parent, child)?;

    self.visit_group_entry(&gr.entry)
  }

  fn visit_type(&mut self, t: &'b Type<'a>) -> visitor::Result<Error> {
    let parent = self.arena_tree.node(CDDLType::Type(t));

    for tc in t.type_choices.iter() {
      let child = self.arena_tree.node(CDDLType::TypeChoice(tc));
      self.insert(parent, child)?;

      self.visit_type_choice(tc)?;
    }

    Ok(())
  }

  fn visit_type_choice(&mut self, tc: &'a TypeChoice<'a>) -> visitor::Result<Error> {
    let parent = self.arena_tree.node(CDDLType::TypeChoice(tc));

    let child = self.arena_tree.node(CDDLType::Type1(&tc.type1));
    self.insert(parent, child)?;

    self.visit_type1(&tc.type1)
  }

  fn visit_type1(&mut self, t1: &'b Type1<'a>) -> visitor::Result<Error> {
    let parent = self.arena_tree.node(CDDLType::Type1(t1));

    if let Some(operator) = &t1.operator {
      let child = self.arena_tree.node(CDDLType::Operator(operator));
      self.insert(parent, child)?;

      self.visit_operator(t1, operator)?;
    }

    let child = self.arena_tree.node(CDDLType::Type2(&t1.type2));
    self.insert(parent, child)?;

    self.visit_type2(&t1.type2)
  }

  fn visit_operator(
    &mut self,
    target: &'b Type1<'a>,
    o: &'b Operator<'a>,
  ) -> visitor::Result<Error> {
    let parent = self.arena_tree.node(CDDLType::Operator(o));
    let child = self.arena_tree.node(CDDLType::Type2(&o.type2));
    self.insert(parent, child)?;

    walk_operator(self, target, o)
  }

  fn visit_type2(&mut self, t2: &'b Type2<'a>) -> visitor::Result<Error> {
    let parent = self.arena_tree.node(CDDLType::Type2(t2));

    match t2 {
      Type2::IntValue { value, .. } => {
        let child = self.arena_tree.node(CDDLType::Value(Value::INT(*value)));
        self.insert(parent, child)?;
      }
      Type2::UintValue { value, .. } => {
        let child = self.arena_tree.node(CDDLType::Value(Value::UINT(*value)));
        self.insert(parent, child)?;
      }
      Type2::FloatValue { value, .. } => {
        let child = self.arena_tree.node(CDDLType::Value(Value::FLOAT(*value)));
        self.insert(parent, child)?;
      }
      Type2::TextValue { value, .. } => {
        let child = self
          .arena_tree
          .node(CDDLType::Value(Value::TEXT(Cow::Borrowed(value))));
        self.insert(parent, child)?;
      }
      Type2::UTF8ByteString { value, .. } => {
        let child = self
          .arena_tree
          .node(CDDLType::Value(Value::BYTE(ByteValue::UTF8(
            Cow::Borrowed(value),
          ))));
        self.insert(parent, child)?;
      }
      Type2::B16ByteString { value, .. } => {
        let child = self
          .arena_tree
          .node(CDDLType::Value(Value::BYTE(ByteValue::B16(Cow::Borrowed(
            value,
          )))));
        self.insert(parent, child)?;
      }
      Type2::B64ByteString { value, .. } => {
        let child = self
          .arena_tree
          .node(CDDLType::Value(Value::BYTE(ByteValue::B64(Cow::Borrowed(
            value,
          )))));
        self.insert(parent, child)?;
      }
      Type2::Typename {
        ident,
        generic_args,
        ..
      } => {
        let child = self.arena_tree.node(CDDLType::Identifier(ident));
        self.insert(parent, child)?;

        if let Some(generic_args) = generic_args {
          let child = self.arena_tree.node(CDDLType::GenericArgs(generic_args));
          self.insert(parent, child)?;

          self.visit_generic_args(generic_args)?;
        }
      }
      Type2::ParenthesizedType { pt, .. } => {
        let child = self.arena_tree.node(CDDLType::Type(pt));
        self.insert(parent, child)?;

        self.visit_type(pt)?;
      }
      Type2::Map { group, .. } => {
        let child = self.arena_tree.node(CDDLType::Group(group));
        self.insert(parent, child)?;

        self.visit_group(group)?;
      }
      Type2::Array { group, .. } => {
        let child = self.arena_tree.node(CDDLType::Group(group));
        self.insert(parent, child)?;

        self.visit_group(group)?;
      }
      Type2::Unwrap { ident, .. } => {
        let child = self.arena_tree.node(CDDLType::Identifier(ident));
        self.insert(parent, child)?;

        self.visit_identifier(ident)?;
      }
      Type2::ChoiceFromInlineGroup { group, .. } => {
        let child = self.arena_tree.node(CDDLType::Group(group));
        self.insert(parent, child)?;

        self.visit_group(group)?;
      }
      Type2::ChoiceFromGroup {
        ident,
        generic_args,
        ..
      } => {
        let child = self.arena_tree.node(CDDLType::Identifier(ident));
        self.insert(parent, child)?;

        if let Some(generic_args) = generic_args {
          let child = self.arena_tree.node(CDDLType::GenericArgs(generic_args));
          self.insert(parent, child)?;

          self.visit_generic_args(generic_args)?;
        }
      }
      Type2::TaggedData { t, .. } => {
        let child = self.arena_tree.node(CDDLType::Type(t));
        self.insert(parent, child)?;

        self.visit_type(t)?;
      }
      _ => (),
    }

    Ok(())
  }

  fn visit_group(&mut self, g: &'b Group<'a>) -> visitor::Result<Error> {
    let parent = self.arena_tree.node(CDDLType::Group(g));

    for gc in g.group_choices.iter() {
      let child = self.arena_tree.node(CDDLType::GroupChoice(gc));
      self.insert(parent, child)?;

      self.visit_group_choice(gc)?;
    }

    Ok(())
  }

  fn visit_group_choice(&mut self, gc: &'b GroupChoice<'a>) -> visitor::Result<Error> {
    let parent = self.arena_tree.node(CDDLType::GroupChoice(gc));

    for (ge, _) in gc.group_entries.iter() {
      let child = self.arena_tree.node(CDDLType::GroupEntry(ge));
      self.insert(parent, child)?;

      self.visit_group_entry(ge)?;
    }

    Ok(())
  }

  fn visit_group_entry(&mut self, entry: &'b GroupEntry<'a>) -> visitor::Result<Error> {
    let parent = self.arena_tree.node(CDDLType::GroupEntry(entry));

    match entry {
      GroupEntry::ValueMemberKey { ge, .. } => {
        let child = self.arena_tree.node(CDDLType::ValueMemberKeyEntry(ge));
        self.insert(parent, child)?;

        self.visit_value_member_key_entry(ge)?;
      }
      GroupEntry::TypeGroupname { ge, .. } => {
        let child = self.arena_tree.node(CDDLType::TypeGroupnameEntry(ge));
        self.insert(parent, child)?;

        self.visit_type_groupname_entry(ge)?;
      }
      GroupEntry::InlineGroup { occur, group, .. } => {
        if let Some(occur) = occur {
          let child = self.arena_tree.node(CDDLType::Occurrence(occur));
          self.insert(parent, child)?;

          self.visit_occurrence(occur)?;
        }

        let child = self.arena_tree.node(CDDLType::Group(group));
        self.insert(parent, child)?;

        self.visit_group(group)?;
      }
    }

    Ok(())
  }

  fn visit_value_member_key_entry(
    &mut self,
    entry: &'b ValueMemberKeyEntry<'a>,
  ) -> visitor::Result<Error> {
    let parent = self.arena_tree.node(CDDLType::ValueMemberKeyEntry(entry));

    if let Some(occur) = &entry.occur {
      let child = self.arena_tree.node(CDDLType::Occurrence(occur));
      self.insert(parent, child)?;

      self.visit_occurrence(occur)?;
    }

    if let Some(mk) = &entry.member_key {
      let child = self.arena_tree.node(CDDLType::MemberKey(mk));
      self.insert(parent, child)?;

      self.visit_memberkey(mk)?;
    }

    let child = self.arena_tree.node(CDDLType::Type(&entry.entry_type));
    self.insert(parent, child)?;

    self.visit_type(&entry.entry_type)
  }

  fn visit_type_groupname_entry(
    &mut self,
    entry: &'b TypeGroupnameEntry<'a>,
  ) -> visitor::Result<Error> {
    let parent = self.arena_tree.node(CDDLType::TypeGroupnameEntry(entry));

    if let Some(o) = &entry.occur {
      let child = self.arena_tree.node(CDDLType::Occurrence(o));
      self.insert(parent, child)?;

      self.visit_occurrence(o)?;
    }

    if let Some(ga) = &entry.generic_args {
      let child = self.arena_tree.node(CDDLType::GenericArgs(ga));
      self.insert(parent, child)?;

      self.visit_generic_args(ga)?;
    }

    let child = self.arena_tree.node(CDDLType::Identifier(&entry.name));
    self.insert(parent, child)?;

    self.visit_identifier(&entry.name)
  }

  fn visit_inline_group_entry(
    &mut self,
    occur: Option<&'b Occurrence<'a>>,
    g: &'b Group<'a>,
  ) -> visitor::Result<Error> {
    let parent = self.arena_tree.node(CDDLType::Group(g));

    if let Some(o) = occur {
      self.visit_occurrence(o)?;
    }

    for gc in g.group_choices.iter() {
      let child = self.arena_tree.node(CDDLType::GroupChoice(gc));
      self.insert(parent, child)?;
    }

    self.visit_group(g)
  }

  fn visit_occurrence(&mut self, o: &'b Occurrence<'a>) -> visitor::Result<Error> {
    let parent = self.arena_tree.node(CDDLType::Occurrence(o));
    let child = self.arena_tree.node(CDDLType::Occur(o.occur));
    self.insert(parent, child)?;

    Ok(())
  }

  fn visit_memberkey(&mut self, mk: &'b MemberKey<'a>) -> visitor::Result<Error> {
    let parent = self.arena_tree.node(CDDLType::MemberKey(mk));

    match mk {
      MemberKey::Type1 { t1, .. } => {
        let child = self.arena_tree.node(CDDLType::Type1(t1));
        self.insert(parent, child)?;

        self.visit_type1(t1)
      }
      MemberKey::Bareword { ident, .. } => {
        let child = self.arena_tree.node(CDDLType::Identifier(ident));
        self.insert(parent, child)?;

        self.visit_identifier(ident)
      }
      MemberKey::Value { value, .. } => {
        let child = self.arena_tree.node(CDDLType::Value(value.to_owned()));
        self.insert(parent, child)?;

        self.visit_value(value)
      }
      MemberKey::NonMemberKey { non_member_key, .. } => {
        let child = self.arena_tree.node(CDDLType::NonMemberKey(non_member_key));
        self.insert(parent, child)?;

        self.visit_nonmemberkey(non_member_key)
      }
    }
  }

  fn visit_generic_args(&mut self, args: &'b GenericArgs<'a>) -> visitor::Result<Error> {
    let parent = self.arena_tree.node(CDDLType::GenericArgs(args));

    for arg in args.args.iter() {
      let child = self.arena_tree.node(CDDLType::GenericArg(arg));
      self.insert(parent, child)?;

      self.visit_generic_arg(arg)?;
    }

    Ok(())
  }

  fn visit_generic_arg(&mut self, arg: &'b GenericArg<'a>) -> visitor::Result<Error> {
    let parent = self.arena_tree.node(CDDLType::GenericArg(arg));
    let child = self.arena_tree.node(CDDLType::Type1(&arg.arg));
    self.insert(parent, child)?;

    self.visit_type1(&arg.arg)
  }

  fn visit_generic_params(&mut self, params: &'b GenericParams<'a>) -> visitor::Result<Error> {
    let parent = self.arena_tree.node(CDDLType::GenericParams(params));

    for param in params.params.iter() {
      let child = self.arena_tree.node(CDDLType::GenericParam(param));
      self.insert(parent, child)?;

      self.visit_generic_param(param)?;
    }

    Ok(())
  }

  fn visit_generic_param(&mut self, param: &'b GenericParam<'a>) -> visitor::Result<Error> {
    let parent = self.arena_tree.node(CDDLType::GenericParam(param));
    let child = self.arena_tree.node(CDDLType::Identifier(&param.param));
    self.insert(parent, child)?;

    self.visit_identifier(&param.param)
  }

  fn visit_nonmemberkey(&mut self, nmk: &'b NonMemberKey<'a>) -> visitor::Result<Error> {
    let parent = self.arena_tree.node(CDDLType::NonMemberKey(nmk));

    match nmk {
      NonMemberKey::Group(group) => {
        let child = self.arena_tree.node(CDDLType::Group(group));
        self.insert(parent, child)?;

        self.visit_group(group)
      }
      NonMemberKey::Type(t) => {
        let child = self.arena_tree.node(CDDLType::Type(t));
        self.insert(parent, child)?;

        self.visit_type(t)
      }
    }
  }
}

#[cfg(test)]
#[cfg(not(target_arch = "wasm32"))]
mod tests {
  #![allow(unused_imports)]

  use crate::cddl_from_str;

  use super::*;

  #[test]
  fn testing() {
    let c = cddl_from_str(
      r#"a = b
    b = "test""#,
      true,
    )
    .unwrap();
    let mut t = ParentVisitor::new(&c);
    t.visit_cddl(&c).unwrap();
  }
}
