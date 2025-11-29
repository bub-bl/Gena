use std::sync::{Arc, Mutex};

use winit::{
    event_loop::ActiveEventLoop,
    window::{WindowAttributes, WindowId},
};

use crate::Window;

pub trait WindowFactory {
    fn create(
        winit_window: winit::window::Window,
    ) -> impl Future<Output = Result<Self, Box<dyn std::error::Error>>>
    where
        Self: Sized;
}

#[derive(Default)]
pub struct WindowManager {
    pub windows: Vec<Arc<Mutex<dyn Window>>>,
    pub active_window: Option<Arc<Mutex<dyn Window>>>,
}

impl WindowManager {
    pub fn new() -> Self {
        Self {
            windows: Vec::new(),
            active_window: None,
        }
    }

    // Méthode générique pour créer n'importe quel type de fenêtre
    pub async fn create_window<W>(
        &mut self,
        event_loop: &ActiveEventLoop,
    ) -> Result<Arc<Mutex<W>>, Box<dyn std::error::Error>>
    where
        W: Window + 'static,
        W: WindowFactory, // Trait pour créer des fenêtres
    {
        let winit_window = event_loop
            .create_window(WindowAttributes::default())
            .map_err(|e| format!("Impossible de créer la fenêtre: {}", e))?;

        let window = W::create(winit_window).await?;
        let window = Arc::new(Mutex::new(window));

        // Cast vers le trait Window pour l'ajouter à la liste générale
        let window_as_trait: Arc<Mutex<dyn Window>> = window.clone();
        self.windows.push(window_as_trait.clone());

        // Définir comme fenêtre active
        self.active_window = Some(window_as_trait);

        Ok(window)
    }

    pub fn remove_window(&mut self, window_id: WindowId) {
        self.windows.retain(|w| {
            match w.lock() {
                Ok(guard) => guard.id() != window_id,
                Err(_) => false, // Supprimer les fenêtres avec des mutex empoisonnés
            }
        });

        // // Vérifier et nettoyer active_window si nécessaire
        // if let Some(active) = &self.active_window {
        //     match active.lock() {
        //         Ok(guard) if guard.id() == window_id => {
        //             self.active_window = None;
        //         }
        //         Err(_) => {
        //             // Mutex empoisonné, nettoyer
        //             self.active_window = None;
        //         }
        //         _ => {} // La fenêtre active n'est pas celle supprimée
        //     }
        // }
    }

    pub fn set_active_window(&mut self, window: Arc<Mutex<dyn Window>>) {
        self.active_window = Some(window);
    }

    pub fn get_active_window(&self) -> Option<Arc<Mutex<dyn Window>>> {
        self.active_window.clone()
    }

    pub fn get_window(&self, window_id: WindowId) -> Option<Arc<Mutex<dyn Window>>> {
        self.windows
            .iter()
            .find(|w| {
                match w.lock() {
                    Ok(guard) => guard.id() == window_id,
                    Err(_) => false, // Ignorer les mutex empoisonnés
                }
            })
            .cloned()
    }

    pub fn window_count(&self) -> usize {
        self.windows.len()
    }

    pub fn has_windows(&self) -> bool {
        !self.windows.is_empty()
    }

    pub fn cleanup_poisoned_windows(&mut self) {
        self.windows.retain(|w| w.lock().is_ok());

        if let Some(active) = &self.active_window {
            if active.lock().is_err() {
                self.active_window = None;
            }
        }
    }

    // Méthode pour itérer sur toutes les fenêtres
    pub fn iter_windows(&self) -> impl Iterator<Item = &Arc<Mutex<dyn Window>>> {
        self.windows.iter()
    }

    // Méthode pour gérer le redraw de toutes les fenêtres
    pub fn handle_redraw_all(&mut self) {
        for window in &self.windows {
            if let Ok(mut guard) = window.lock() {
                guard.handle_redraw();
            }
        }
    }

    // Méthode pour gérer les événements de redimensionnement
    pub fn handle_window_resized(&mut self, window_id: WindowId, width: u32, height: u32) {
        if let Some(window) = self.get_window(window_id) {
            if let Ok(mut guard) = window.lock() {
                guard.handle_resized(width, height);
            }
        }
    }

    // Méthode pour gérer le redraw d'une fenêtre spécifique
    pub fn handle_window_redraw(&mut self, window_id: WindowId) {
        if let Some(window) = self.get_window(window_id) {
            if let Ok(mut guard) = window.lock() {
                guard.handle_redraw();
            }
        }
    }

    // Méthode pour fermer toutes les fenêtres
    pub fn close_all_windows(&mut self) {
        self.windows.clear();
        self.active_window = None;
    }

    // Sélectionner la prochaine fenêtre comme active
    pub fn select_next_active_window(&mut self) {
        if self.windows.is_empty() {
            self.active_window = None;
            return;
        }

        // Si aucune fenêtre active, prendre la première
        if self.active_window.is_none() {
            self.active_window = self.windows.first().cloned();
            return;
        }

        // Obtenir l'ID de la fenêtre active actuelle sans garder le lock
        let active_id = self
            .active_window
            .as_ref()
            .and_then(|active| active.lock().ok().map(|guard| guard.id()));

        if let Some(active_id) = active_id
            && let Some(current_index) = self.windows.iter().position(|w| {
                w.lock()
                    .map(|guard| guard.id() == active_id)
                    .unwrap_or(false)
            })
        {
            let next_index = (current_index + 1) % self.windows.len();
            // Ici le lock précédent est déjà tombé, donc on peut assigner
            self.active_window = self.windows.get(next_index).cloned();
        }
    }
}
