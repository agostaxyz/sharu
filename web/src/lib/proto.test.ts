import { describe, expect, it } from 'vitest';
import client from '../../../crates/proto/fixtures/client.json';
import server from '../../../crates/proto/fixtures/server.json';
import { CLIENT_TYPES, SERVER_TYPES } from './proto';

// Contract test: every fixture the Rust side round-trips must use a type the TS side knows,
// and every TS type must be covered by a fixture.
describe('wire protocol contract', () => {
  it('client fixtures match TS types', () => {
    const types = new Set(client.map((m) => m.type));
    expect([...types].sort()).toEqual([...CLIENT_TYPES].sort());
  });
  it('server fixtures match TS types', () => {
    const types = new Set(server.map((m) => m.type));
    expect([...types].sort()).toEqual([...SERVER_TYPES].sort());
  });
});
