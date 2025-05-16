import { CommonModule } from '@angular/common'
import { Component, effect, OnInit, signal, WritableSignal } from '@angular/core'
import { FormsModule, ReactiveFormsModule } from '@angular/forms'
import { RouterOutlet } from '@angular/router'
import { invoke } from '@tauri-apps/api/core'
import { open } from '@tauri-apps/plugin-dialog'

@Component({
  selector: 'app-root',
  standalone: true,
  imports: [CommonModule, RouterOutlet, FormsModule, ReactiveFormsModule],
  templateUrl: './app.component.html',
  styleUrl: './app.component.css',
})
export class AppComponent implements OnInit {
  folders: { value: string, label: string }[] = []
  selectedFolder: WritableSignal<{ value: string, label: string }> = signal({ value: '', label: '' })
  files: { value: string, label: string }[] = []
  audioDevices: string[] = []
  selectedAudioDevice: string = ''
  // between 0 and 1
  globalVolume: number = 1

  constructor() {
    effect(() => {
      this.selectFolder(this.selectedFolder())
    })
  }

  async ngOnInit(): Promise<void> {
    await this.loadFolders()
    await this.loadAudioDevices()
  }

  async loadFolders(): Promise<void> {
    const folders = await invoke<string[]>('get_folders')
    console.log(folders)
    this.folders = folders.map(folder => ({ value: folder, label: folder.split(/[/\\]/).pop() || folder }))
  }

  async loadAudioDevices(): Promise<void> {
    this.audioDevices = await invoke<string[]>('list_audio_devices')
    if (this.audioDevices.length > 0) {
      this.selectedAudioDevice = this.audioDevices[0] // Select the first device by default
    }
    console.log('Audio Devices:', this.audioDevices)
  }

  // folder picker

  addNewFolder() {
    open({
      directory: true,
      multiple: true,
    }).then((res) => {
      console.log(res)
      if (res) {
        invoke('add_folders', { folders: res })
      }
    })
  }

  async selectFolder(folder: { value: string, label: string }) {
    if (!folder.value) return
    const files = await invoke<string[]>('get_files_in_folder', { folder: folder.value })
    this.files = files.map((file) => {
      return { value: file, label: file.split(/[/\\]/).pop() || file }
    })
  }

  async play(file: string) {
    await invoke('play_audio', { path: file })
  }

  async setAudioDevice(device: string): Promise<void> {
    try {
      await invoke('set_audio_device', { deviceName: device })
      this.selectedAudioDevice = device
      console.log(`Successfully set audio device to ${device}`)
    }
    catch (error) {
      console.error(`Failed to set audio device to ${device}:`, error)
      // Handle error appropriately (e.g., display an error message to the user)
    }
  }

  async setVolume(volume: number): Promise<void> {
    try {
      await invoke('set_volume', { volume })
      this.globalVolume = volume
      console.log(`Successfully set global volume to ${volume}`)
    }
    catch (error) {
      console.error(`Failed to set global volume to ${volume}:`, error)
      // Handle error appropriately (e.g., display an error message to the user)
    }
  }
}
