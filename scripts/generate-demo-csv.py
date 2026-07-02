#!/usr/bin/env python3
"""Génère clients.csv et vehicules_articles.csv avec N lignes de données (hors en-tête)."""

from __future__ import annotations

import random
import string
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
TARGET_ROWS = 30_000
SEED = 42

PRENOMS = [
    "David", "Chloe", "Julie", "Mathieu", "Christophe", "Anne", "Hugo", "Bruno", "Patricia",
    "Philippe", "Michel", "Francois", "Lea", "Stephane", "Marion", "Nicolas", "Sophie",
    "Thomas", "Emilie", "Antoine", "Camille", "Pierre", "Florian", "Loic", "Nadege",
    "Noemie", "Laura", "Vincent", "Isabelle", "Olivier", "Celine", "Jerome", "Sandrine",
    "Guillaume", "Aurelie", "Fabien", "Valerie", "Sebastien", "Caroline", "Maxime",
    "Benjamin", "Charlotte", "Alexandre", "Manon", "Romain", "Elodie", "Julien", "Pauline",
]

NOMS = [
    "Laurent", "Roux", "Fournier", "Morel", "Martinez", "Dupont", "Moreau", "Girard",
    "Lambert", "David", "Vincent", "Dubois", "Robert", "Richard", "Petit", "Durand",
    "Leroy", "Simon", "Michel", "Garcia", "Bernard", "Thomas", "Rousseau", "Fontaine",
    "Chevalier", "Robin", "Bertrand", "Morin", "Gauthier", "Perez", "Lopez", "Caron",
    "Sanchez", "Guillot", "Roger", "Blanc", "Henry", "Masson", "Nicolas", "Perrot",
]

VILLES = [
    "Paris", "Lyon", "Marseille", "Toulouse", "Nice", "Nantes", "Strasbourg", "Montpellier",
    "Bordeaux", "Lille", "Rennes", "Reims", "Le Havre", "Saint-Etienne", "Toulon", "Grenoble",
    "Dijon", "Angers", "Nimes", "Villeurbanne", "Clermont-Ferrand", "Le Mans", "Aix-en-Provence",
    "Brest", "Tours", "Amiens", "Limoges", "Annecy", "Perpignan", "Besancon", "Metz", "Rouen",
    "Caen", "Mulhouse", "Valence", "Hyeres", "Martigues",
]

PROFESSIONS = [
    "Comptable", "Cadre", "Medecin", "Enseignant", "Consultant", "Retraite", "Technicien",
    "Avocat", "Ingenieur", "Agent immobilier", "Infirmier", "Commercant", "Plombier",
    "Developpeur", "Veterinaire", "Couvreur", "Architecte", "Pharmacien", "Artisan",
    "Employe", "Chef de projet", "Designer", "Electricien", "Agriculteur",
]

MARQUES_MODELES: list[tuple[str, list[str]]] = [
    ("Renault", ["Clio V", "Megane E-Tech", "Captur", "Arkana", "Scenic E-Tech", "Kadjar", "Twingo"]),
    ("Peugeot", ["208", "308", "3008", "5008", "2008", "408"]),
    ("Citroen", ["C3", "C4", "C5 Aircross", "Berlingo"]),
    ("Volkswagen", ["Polo", "Golf", "T-Roc", "Tiguan", "ID.3", "Passat"]),
    ("Toyota", ["Yaris Hybrid", "Corolla", "RAV4", "C-HR", "Aygo X"]),
    ("Ford", ["Fiesta", "Focus", "Puma", "Kuga", "Mustang Mach-E"]),
    ("Hyundai", ["i20", "i30", "Tucson", "Kona", "Ioniq 5"]),
    ("Kia", ["Picanto", "Ceed", "Sportage", "Niro", "EV6"]),
    ("Dacia", ["Sandero", "Duster", "Jogger", "Spring"]),
    ("Nissan", ["Micra", "Juke", "Qashqai", "Leaf"]),
    ("Seat", ["Ibiza", "Leon", "Arona", "Ateca"]),
    ("Opel", ["Corsa", "Astra", "Mokka", "Grandland"]),
    ("Fiat", ["500", "Panda", "Tipo", "600"]),
    ("Mini", ["Cooper", "Countryman", "Clubman"]),
    ("BMW", ["Serie 1", "Serie 3", "X1", "X3"]),
    ("Mercedes", ["Classe A", "Classe C", "GLA", "GLC"]),
    ("Audi", ["A1", "A3", "Q2", "Q3", "Q5"]),
]

COULEURS = [
    "Rouge flamme", "Bleu intense", "Argent", "Beige sable", "Blanc nacre", "Gris platine",
    "Vert olive", "Violet royal", "Noir perla", "Orange fusion",
]

CARBURANTS = ["Essence", "Diesel", "Hybride", "Electrique", "GPL"]


def client_ref(i: int) -> str:
    return f"CLI-{i:02d}" if i < 10 else f"CLI-{i}"


def veh_ref(i: int) -> str:
    return f"VEH-{i:03d}" if i < 1000 else f"VEH-{i}"


def phone(rng: random.Random) -> str:
    parts = [f"{rng.randint(10, 99):02d}" for _ in range(4)]
    return f"06 {' '.join(parts)}"


def immat(rng: random.Random) -> str:
    d1 = rng.randint(10, 99)
    letters = "".join(rng.choice(string.ascii_uppercase) for _ in range(3))
    d2 = rng.randint(100, 999)
    return f"{d1}-{letters}-{d2}"


def norm_email_part(s: str) -> str:
    return (
        s.lower()
        .replace(" ", "")
        .replace("é", "e")
        .replace("è", "e")
        .replace("ê", "e")
        .replace("à", "a")
        .replace("ô", "o")
        .replace("ç", "c")
    )


def generate_clients(rng: random.Random, count: int) -> list[str]:
    lines = ["reference|prenom|nom|age|ville|profession|email|telephone"]
    for i in range(1, count + 1):
        prenom = rng.choice(PRENOMS)
        nom = rng.choice(NOMS)
        age = rng.randint(18, 75)
        ville = rng.choice(VILLES)
        profession = rng.choice(PROFESSIONS)
        email = f"{norm_email_part(prenom)}.{norm_email_part(nom)}{i}@mail.fr"
        lines.append(
            f"{client_ref(i)}|{prenom}|{nom}|{age}|{ville}|{profession}|{email}|{phone(rng)}"
        )
    return lines


def generate_vehicules(rng: random.Random, count: int) -> list[str]:
    lines = [
        "reference|marque|modele|annee|couleur|carburant|kilometrage|immatriculation|ville|prix|qte_initial"
    ]
    for i in range(1, count + 1):
        marque, modeles = rng.choice(MARQUES_MODELES)
        modele = rng.choice(modeles)
        annee = rng.randint(2015, 2025)
        couleur = rng.choice(COULEURS)
        carburant = rng.choice(CARBURANTS)
        km = rng.randint(5_000, 200_000)
        immatriculation = immat(rng)
        ville = rng.choice(VILLES)
        prix = rng.randint(8_000, 60_000)
        qte = rng.randint(1, 8)
        lines.append(
            f"{veh_ref(i)}|{marque}|{modele}|{annee}|{couleur}|{carburant}|{km}|"
            f"{immatriculation}|{ville}|{prix}|{qte}"
        )
    return lines


def main() -> None:
    rng = random.Random(SEED)
    clients_path = ROOT / "public" / "clients.csv"
    vehicules_path = ROOT / "public" / "vehicules_articles.csv"

    clients_path.write_text("\n".join(generate_clients(rng, TARGET_ROWS)) + "\n", encoding="utf-8")
    vehicules_path.write_text("\n".join(generate_vehicules(rng, TARGET_ROWS)) + "\n", encoding="utf-8")

    print(f"clients.csv : {TARGET_ROWS} lignes de donnees -> {clients_path}")
    print(f"vehicules_articles.csv : {TARGET_ROWS} lignes de donnees -> {vehicules_path}")


if __name__ == "__main__":
    main()
