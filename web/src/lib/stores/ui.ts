import { writable } from 'svelte/store';

export type RoomsModalTab = 'join' | 'create';

export interface UiState {
  roomsModalOpen: boolean;
  roomsModalTab: RoomsModalTab;
  copiedToken: boolean;
}

function createUiStore() {
  const { subscribe, update } = writable<UiState>({
    roomsModalOpen: true,
    roomsModalTab: 'join',
    copiedToken: false,
  });

  return {
    subscribe,
    openRoomsModal: (tab: RoomsModalTab = 'join') => update((state) => ({ ...state, roomsModalOpen: true, roomsModalTab: tab })),
    closeRoomsModal: () => update((state) => ({ ...state, roomsModalOpen: false })),
    setRoomsModalTab: (tab: RoomsModalTab) => update((state) => ({ ...state, roomsModalTab: tab })),
    setCopiedToken: (copiedToken: boolean) => update((state) => ({ ...state, copiedToken })),
  };
}

export const uiStore = createUiStore();
