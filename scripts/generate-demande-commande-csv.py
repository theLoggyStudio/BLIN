#!/usr/bin/env python3
"""Génère demande_dachat.csv et commande.csv (50 lignes non signées)."""

from __future__ import annotations

import csv
import json
import random
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
PUBLIC = ROOT / "public"
SEED = 42


def load_clients(limit: int = 50) -> list[dict[str, str]]:
    path = PUBLIC / "clients.csv"
    rows: list[dict[str, str]] = []
    with path.open(encoding="utf-8") as f:
        reader = csv.DictReader(f, delimiter="|")
        for i, row in enumerate(reader):
            if i >= limit:
                break
            rows.append(row)
    return rows


def main() -> None:
    rng = random.Random(SEED)
    clients = load_clients(50)

    da_lines = [
        "client_reference|client_prenom|client_nom|client_age|client_ville|client_profession|client_email|client_telephone|articles|statut_signature"
    ]
    for i, c in enumerate(clients, 1):
        veh_ref = f"VEH-{i:03d}"
        articles = json.dumps(
            [{"reference": veh_ref, "qte_initial": str(rng.randint(1, 3))}],
            ensure_ascii=False,
        )
        da_lines.append(
            "|".join(
                [
                    c["reference"],
                    c["prenom"],
                    c["nom"],
                    c["age"],
                    c["ville"],
                    c["profession"],
                    c["email"],
                    c["telephone"],
                    articles,
                    "non_signe",
                ]
            )
        )

    cmd_lines = ["matricule_libelle|da|statut_signature"]
    for i, _c in enumerate(clients, 1):
        veh_ref = f"VEH-{i:03d}"
        da = json.dumps([{"reference": veh_ref, "qte_initial": "1"}], ensure_ascii=False)
        cmd_lines.append(f"CMD-{i:02d}|{da}|non_signe")

    (PUBLIC / "demande_dachat.csv").write_text("\n".join(da_lines) + "\n", encoding="utf-8")
    (PUBLIC / "commande.csv").write_text("\n".join(cmd_lines) + "\n", encoding="utf-8")

    print(f"demande_dachat.csv : {len(da_lines) - 1} lignes -> {PUBLIC / 'demande_dachat.csv'}")
    print(f"commande.csv : {len(cmd_lines) - 1} lignes -> {PUBLIC / 'commande.csv'}")


if __name__ == "__main__":
    main()
