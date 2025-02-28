use std::{collections::HashMap, path::PathBuf};

use regex::Regex;

pub struct Job {
    pub job_id: String,
    pub array_id: String,
    pub array_step: Option<String>,
    pub name: String,
    pub state: String,
    pub state_compact: String,
    pub reason: Option<String>,
    pub user: String,
    pub time: String,
    pub tres: String,
    pub partition: String,
    pub nodelist: String,
    pub stdout: Option<PathBuf>,
    pub stderr: Option<PathBuf>,
    pub command: String,
}

impl Job {
    pub fn id(&self) -> String {
        match self.array_step.as_ref() {
            Some(array_step) => format!("{}_{}", self.array_id, array_step),
            None => self.job_id.clone(),
        }
    }
    ///
    /// # Arguments
    /// * `squeue_l` - A vector of strings representing the fields of the job.
    /// * `fields` - A slice of strings representing the names of the fields in the job.
    /// * `output_separator` - A string representing the separator used in the output of the squeue command.
    pub fn from_parts(squeue_l: String,
        fields: &[&str],
        output_separator: &str) -> Option<Self> {

        let parts = squeue_l.split(output_separator).collect::<Vec<_>>();
        // Check that they are both the same length
        if parts.len() != fields.len() + 1 {
            return None;
        }
        // Create a HashMap with the field names as keys and the field values as values
        let mut field_values = HashMap::<&str,String>::new();
        for (field, part) in fields.iter().zip(parts.into_iter()) {
            field_values.insert(field, part.to_string());
        }
        
        // Try to follow order for Job struct
        let job_id = field_values.get("jobid").unwrap().clone();
        let array_id = field_values.get("ArrayJobID").unwrap().clone();
        let array_task_id = field_values.get("ArrayTaskID").unwrap();
        let array_step = if array_task_id == "N/A" {
            None
        } else {
            Some(array_task_id.to_owned())
        };
        let name = field_values.get("name").unwrap().clone();
        let state = field_values.get("state").unwrap().clone();
        let state_compact = field_values.get("statecompact").unwrap().clone();
        let reason = field_values.get("reason")
        .filter(|&value| value != "None").cloned();
        let user = field_values.get("username").unwrap().clone();
        let time = field_values.get("timeused").unwrap().clone();
        let cpu_tres = field_values.get("tres-alloc").unwrap();
        let tres_per_node = field_values.get("tres-per-node").unwrap();
        // add gpu to tres
        let tres: String = if tres_per_node.as_str() == "N/A" {
            cpu_tres.to_string()
        } else {
            format!("{},{}", cpu_tres, tres_per_node)
        };
        let partition = field_values.get("partition").unwrap().clone();
        let nodelist = field_values.get("NodeList").unwrap().clone();
        let command = field_values.get("command").unwrap().clone();
        let stdout_path = field_values.get("stdout").unwrap();
        let stdout = Self::resolve_path(
            stdout_path,
            &field_values,
        );
        let stderr_path = field_values.get("stderr").unwrap();
        let stderr = Self::resolve_path(
            stderr_path,
            &field_values,
        );

        Some(Job {
            job_id,
            array_id,
            array_step,
            name,
            state,
            state_compact,
            reason,
            user,
            time,
            tres,
            partition,
            nodelist,
            command,
            stdout,
            stderr,
        }
        )

    }

    
    fn resolve_path(
        path: &str,
        field_values: &HashMap<&str,String>,
    ) -> Option<PathBuf> {
        // see https://slurm.schedmd.com/sbatch.html#SECTION_%3CB%3Efilename-pattern%3C/B%3E
        lazy_static::lazy_static! {
            static ref RE: Regex = Regex::new(r"%(%|A|a|J|j|N|n|s|t|u|x)").unwrap();
        }
        let mut path = path.to_owned();
        let array_master = field_values.get("ArrayJobID").unwrap();
        let array_id = field_values.get("ArrayTaskID").unwrap();
        let id = field_values.get("jobid").unwrap();
        let host = field_values.get("NodeList").unwrap();
        let user = field_values.get("username").unwrap();
        let name = field_values.get("name").unwrap();
        let working_dir = field_values.get("WorkDir").unwrap();
        
        let slurm_no_val = "4294967294";
        let array_id = if array_id == "N/A" {
            slurm_no_val
        } else {
            array_id
        };

        if path.is_empty() {
            // never happens right now, because `squeue -O stdout` seems to always return something
            path = if array_id == slurm_no_val {
                PathBuf::from(working_dir).join("slurm-%J.out")
            } else {
                PathBuf::from(working_dir).join("slurm-%A_%a.out")
            }
            .to_str()
            .unwrap()
            .to_owned();
        };
        // Would this be good? Allows ui to handle gracefully
        if path == "(null)" {
            return None;
        }

        for cap in RE
            .captures_iter(&path.clone())
            .collect::<Vec<_>>() // TODO: this is stupid, there has to be a better way to reverse the captures...
            .iter()
            .rev()
        {
            let m = cap.get(0).unwrap();
            let replacement = match m.as_str() {
                "%%" => "%",
                "%A" => array_master,
                "%a" => array_id,
                "%J" => id,
                "%j" => id,
                "%N" => host.split(',').next().unwrap_or(host),
                "%n" => "0",
                "%s" => "batch",
                "%t" => "0",
                "%u" => user,
                "%x" => name,
                _ => unreachable!(),
            };

            path.replace_range(m.range(), replacement);
        }

        Some(PathBuf::from(working_dir).join(path)) // works even if `path` is absolute
    }


}