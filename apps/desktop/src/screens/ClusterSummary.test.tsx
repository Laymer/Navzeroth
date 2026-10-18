import {render,screen} from '@testing-library/react';
import {ClusterSummary} from './ClusterSummary';
import type {ClusteringMetadata} from '../commands/clustering';
it('explains shared inbound travel while retaining additive service and minimum party utility',()=>{
 const clustering:ClusteringMetadata={schema_version:1,selected_clusters:[{members:['quest-a','quest-b'],node:'node-a',participant_utilities:[7,4],utility:4,completion_ms:1000,service_ms:2000,inbound_ms:120000,stopping_ms:0,incremental_ms:123000,expected_session_ms:123000}],cluster_alternatives:[],assessments:[],variant_trace:[],peak_variant_count:1,variants_evaluated:1};
 const view=render(<ClusterSummary clustering={clustering}/>);
 expect(screen.getByLabelText('Objective clustering')).toHaveTextContent('shared kills are not assumed');
 expect(screen.getByRole('listitem')).toHaveTextContent('quest-a, quest-b: 2 minutes inbound travel; minimum participant total utility 4');
 view.rerender(<ClusterSummary clustering={undefined}/>);expect(screen.queryByLabelText('Objective clustering')).not.toBeInTheDocument();
 view.rerender(<ClusterSummary clustering={{...clustering,schema_version:2} as unknown as ClusteringMetadata}/>);expect(screen.queryByLabelText('Objective clustering')).not.toBeInTheDocument();
});
