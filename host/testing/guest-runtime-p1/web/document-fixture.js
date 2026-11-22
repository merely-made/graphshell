// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

// A stand-in document for the supervisor (guest runtime plan, P1). It is NOT
// under test and it is NOT Genet's DOM: it holds the native test page
// (`<body><p>Hello</p></body>`, `page()` in `tests/document_script.rs`) as a
// plain tree, produces the `document-view` the contract describes the way
// `dom_view.rs` fills it, and applies a batch by the rules of `dom_view::apply`
// for the cases the guard tests reach. Node ids are fixture numbers, not
// Genet's. Reusing the real `dom_view.rs` needs a split of `document-host`
// that is deferred.

const FNV_OFFSET = 0xcbf29ce484222325n;
const FNV_PRIME = 0x100000001b3n;
const encoder = new TextEncoder();

/** The change-detection token of `content_hash` in `src/lib.rs`: FNV-1a over kind, 0, text. */
export function contentHash(kind, text) {
  let hash = FNV_OFFSET;
  const bytes = [...encoder.encode(kind), 0, ...encoder.encode(text)];
  for (const byte of bytes) {
    hash = BigInt.asUintN(64, (hash ^ BigInt(byte)) * FNV_PRIME);
  }
  return hash;
}

export class DocumentFixture {
  /** Node id to `{ id, parent, kind, text, children }`. */
  #nodes = new Map();
  #root;
  #nextId = 1n;
  revision = 0n;

  constructor() {
    this.#root = this.#create(null, '#document', '');
    const body = this.#create(this.#root, 'body', '');
    const paragraph = this.#create(body, 'p', '');
    this.#create(paragraph, '#text', 'Hello');
  }

  #create(parent, kind, text) {
    const node = { id: this.#nextId, parent, kind, text, children: [] };
    this.#nextId += 1n;
    this.#nodes.set(node.id, node);
    if (parent !== null) this.#nodes.get(parent).children.push(node.id);
    return node.id;
  }

  /** The nodes in document order. */
  #walk(id, out) {
    const node = this.#nodes.get(id);
    out.push(node);
    for (const child of node.children) this.#walk(child, out);
    return out;
  }

  /** `dom_view::snapshot`: the whole document as a copied view. */
  view() {
    const nodes = this.#walk(this.#root, []).map((node) => ({
      id: node.id,
      parent: node.parent ?? undefined,
      kind: node.kind,
      text: node.text,
      contentHash: contentHash(node.kind, node.text),
    }));
    return { revision: this.revision, nodes };
  }

  /** The text of every text node, in order. */
  texts() {
    return this.#walk(this.#root, []).filter((n) => n.kind === '#text').map((n) => n.text);
  }

  #referenced(change) {
    switch (change.tag) {
      case 'set-text': return change.val.node;
      case 'remove': return change.val;
      case 'insert-before': return change.val.reference;
      case 'append-child': return change.val.parent;
      default: return undefined;
    }
  }

  #drop(id) {
    const node = this.#nodes.get(id);
    for (const child of [...node.children]) this.#drop(child);
    this.#nodes.delete(id);
  }

  #applyOne(change) {
    switch (change.tag) {
      case 'set-text': {
        const node = this.#nodes.get(change.val.node);
        if (node.kind === '#text') node.text = change.val.text;
        break;
      }
      case 'remove': {
        const node = this.#nodes.get(change.val);
        const siblings = this.#nodes.get(node.parent)?.children;
        if (siblings) siblings.splice(siblings.indexOf(node.id), 1);
        this.#drop(node.id);
        break;
      }
      case 'insert-before': {
        const reference = this.#nodes.get(change.val.reference);
        if (reference.parent === null) break;
        const id = this.#create(reference.parent, change.val.new.kind, change.val.new.text);
        const siblings = this.#nodes.get(reference.parent).children;
        siblings.pop();
        siblings.splice(siblings.indexOf(reference.id), 0, id);
        break;
      }
      case 'append-child':
        this.#create(change.val.parent, change.val.new.kind, change.val.new.text);
        break;
      default:
        break;
    }
  }

  /**
   * `dom_view::apply`: a stale revision is a conflict carrying the current
   * one; an unknown node rejects the whole batch; an empty batch leaves the
   * revision alone; otherwise every change applies and the revision advances.
   */
  apply(batch) {
    if (batch.expectedRevision !== this.revision) {
      return { outcome: 'conflict', revision: this.revision };
    }
    for (const change of batch.changes) {
      const id = this.#referenced(change);
      if (id !== undefined && !this.#nodes.has(id)) return { outcome: 'unknownNode', node: id };
    }
    if (batch.changes.length === 0) return { outcome: 'applied', revision: this.revision };
    for (const change of batch.changes) this.#applyOne(change);
    this.revision += 1n;
    return { outcome: 'applied', revision: this.revision };
  }
}
