/**
 * Scene registry: storyboard scene id → the scene module's default export
 * (component, frames relative to the scene start) and its `sfx` cues.
 * src/compositions/Film.tsx mounts scenes and the master audio from here.
 */
import type React from 'react';
import type {SfxCue} from '../shared/audio';
import type {SceneId} from '../storyboard';
import S01HookTerca, {sfx as sfx01} from './s01-hook-terca';
import S02SoUmMinuto, {sfx as sfx02} from './s02-so-um-minuto';
import S03PlanilhaDeMemoria, {sfx as sfx03} from './s03-planilha-de-memoria';
import S04Silencio, {sfx as sfx04} from './s04-silencio';
import S05DropUbiqx, {sfx as sfx05} from './s05-drop-ubiqx';
import S06EleRegistra, {sfx as sfx06} from './s06-ele-registra';
import S07AsSuasCategorias, {sfx as sfx07} from './s07-as-suas-categorias';
import S08RegrasMemoriaIa, {sfx as sfx08} from './s08-regras-memoria-ia';
import S09SoPergunta, {sfx as sfx09} from './s09-so-pergunta';
import S10Tecla1, {sfx as sfx10} from './s10-tecla-1';
import S11EEleAprende, {sfx as sfx11} from './s11-e-ele-aprende';
import S12UmClique, {sfx as sfx12} from './s12-um-clique';
import S13NadaEmDuvida, {sfx as sfx13} from './s13-nada-em-duvida';
import S14Relatorio, {sfx as sfx14} from './s14-relatorio';
import S15Foco, {sfx as sfx15} from './s15-foco';
import S16SuaIa, {sfx as sfx16} from './s16-sua-ia';
import S17Privacidade, {sfx as sfx17} from './s17-privacidade';
import S18EndCard, {sfx as sfx18} from './s18-end-card';

export type SceneModule = {component: React.FC; sfx: SfxCue[]};

export const SCENE_MODULES: Record<SceneId, SceneModule> = {
	's01-hook-terca': {component: S01HookTerca, sfx: sfx01},
	's02-so-um-minuto': {component: S02SoUmMinuto, sfx: sfx02},
	's03-planilha-de-memoria': {component: S03PlanilhaDeMemoria, sfx: sfx03},
	's04-silencio': {component: S04Silencio, sfx: sfx04},
	's05-drop-ubiqx': {component: S05DropUbiqx, sfx: sfx05},
	's06-ele-registra': {component: S06EleRegistra, sfx: sfx06},
	's07-as-suas-categorias': {component: S07AsSuasCategorias, sfx: sfx07},
	's08-regras-memoria-ia': {component: S08RegrasMemoriaIa, sfx: sfx08},
	's09-so-pergunta': {component: S09SoPergunta, sfx: sfx09},
	's10-tecla-1': {component: S10Tecla1, sfx: sfx10},
	's11-e-ele-aprende': {component: S11EEleAprende, sfx: sfx11},
	's12-um-clique': {component: S12UmClique, sfx: sfx12},
	's13-nada-em-duvida': {component: S13NadaEmDuvida, sfx: sfx13},
	's14-relatorio': {component: S14Relatorio, sfx: sfx14},
	's15-foco': {component: S15Foco, sfx: sfx15},
	's16-sua-ia': {component: S16SuaIa, sfx: sfx16},
	's17-privacidade': {component: S17Privacidade, sfx: sfx17},
	's18-end-card': {component: S18EndCard, sfx: sfx18},
};
