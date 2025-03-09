import { CommonModule } from '@angular/common';
import { Component, effect, OnInit, signal, WritableSignal } from '@angular/core';
import { FormsModule, ReactiveFormsModule } from '@angular/forms';
import { RouterOutlet } from '@angular/router';
import { invoke } from "@tauri-apps/api/core";
import { open } from '@tauri-apps/plugin-dialog';

@Component({
  selector: 'app-root',
  standalone: true,
  imports: [CommonModule, RouterOutlet, FormsModule, ReactiveFormsModule],
  templateUrl: './app.component.html',
  styleUrl: './app.component.css'
})
export class AppComponent implements OnInit {
  folders: { value: string, label: string }[] = [];
  selectedFolder: WritableSignal<{ value: string, label: string }> = signal({ value: '', label: '' });
  files: { value: string, label: string }[] = [];

  constructor() {
    effect(() => {
      this.selectFolder(this.selectedFolder());
    })
  }

  ngOnInit(): void {
    invoke<string[]>("get_folders").then((folders) => {
      console.log(folders);
      this.folders = folders.map((folder) => {
        return { value: folder, label: folder.split('/').pop() || folder }
      })
    });
  }

  // folder picker

  addNewFolder() {
    open({
      directory: true,
      multiple: true,
    }).then((res) => {
      console.log(res);
      if (res) {
        invoke("add_folders", { folders: res })
      }
    });
  }

  async selectFolder(folder: { value: string, label: string }) {
    if (!folder.value) return;
    const files = await invoke<string[]>("get_files_in_folder", { folder: folder.value })
    this.files = files.map((file) => {
      return { value: file, label: file.split('/').pop() || file }
    })
  }

  async play(file: string) {
    await invoke("play_audio", { path: file })
  }
}
