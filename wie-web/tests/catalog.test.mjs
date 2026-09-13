import assert from 'node:assert/strict';
import test from 'node:test';
import { matchesGame, groupGames } from '../src/ts/catalog.ts';

test('Korean search normalizes decomposed names and ignores spaces', () => {
  assert.equal(matchesGame({ title: '미니게임 천국2', carrier: 'LGT' }, '미니게임천국'.normalize('NFD'), ''), true);
});
test('carrier variants and numbered sequels remain distinct', () => {
  const game = { title: '리듬스타2', carrier: 'LGT' };
  assert.equal(matchesGame(game, '리듬스타', 'KTF'), false);
  assert.equal(matchesGame(game, '리듬스타1', ''), false);
  assert.equal(matchesGame(game, '', 'LGT'), true);
});
test('Latin names match without case sensitivity', () => {
  assert.equal(matchesGame({ title: 'KBO 프로야구', carrier: 'KTF' }, 'kbo', ''), true);
});

const entry = (id, title, carrier) => ({id, title, carrier, source:`https://example.test/${id}`, thumbnail:null});
test('matching carrier versions become one card without losing launch identities', () => {
  const input = [entry('1','미니게임 천국2','LGT'), entry('2','미니게임천국2'.normalize('NFD'),'KTF'), entry('3','미니게임천국2','SKT')];
  const groups = groupGames(input);
  assert.equal(groups.length,1);
  assert.deepEqual(groups[0].versions.map(game => [game.id,game.carrier]), [['1','LGT'],['2','KTF'],['3','SKT']]);
  assert.equal(input.length,3);
});
test('sequels and named editions remain separate, while a carrier filter narrows choices', () => {
  const input = [entry('1','리듬스타','KTF'),entry('2','리듬스타2','LGT'),entry('3','리듬스타2','KTF'),entry('4','리듬스타2 특별판','LGT')];
  assert.equal(groupGames(input).length,3);
  const filtered = groupGames(input.filter(game => matchesGame(game,'리듬스타2','LGT')));
  assert.equal(filtered.length,2);
  assert.deepEqual(filtered[0].versions.map(game => game.carrier),['LGT']);
});
